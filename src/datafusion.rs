use crate::{FieldType, FieldValue, Reader};
use async_trait::async_trait;
use datafusion::arrow::array::{
    ArrayBuilder, ArrayRef, BooleanBuilder, Date32Builder, Float32Builder, Float64Builder,
    Int32Builder, Int64Builder, StringBuilder,
};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::provider::TableProviderFactory;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::context::{SessionState, TaskContext};
use datafusion::physical_plan::expressions::PhysicalSortExpr;
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::{
    project_schema, DisplayAs, DisplayFormatType, ExecutionPlan, SendableRecordBatchStream,
    Statistics,
};
use datafusion::prelude::*;
use datafusion_expr::CreateExternalTable;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct DbaseTable {
    path: String,
    reader: Arc<Mutex<Reader<BufReader<std::fs::File>>>>,
}

impl Clone for DbaseTable {
    fn clone(&self) -> Self {
        return DbaseTable {
            path: self.path.clone(),
            reader: self.reader.clone(),
        };
    }
}

impl DbaseTable {
    pub fn new<P: AsRef<Path> + Debug>(path: P) -> Self {
        let reader = Reader::from_path(&path)
            .expect(format!("Could not find file {:?} or corresponding memo file", &path).as_str());
        return DbaseTable {
            path: path
                .as_ref()
                .to_str()
                .expect("Path contains non-unicode characters")
                .to_string(),
            reader: Arc::new(Mutex::new(reader)),
        };
    }

    pub fn num_records(&self) -> usize {
        return self
            .reader
            .lock()
            .unwrap()
            .header()
            .num_records
            .try_into()
            .unwrap();
    }

    pub(crate) async fn create_physical_plan(
        &self,
        projections: Option<&Vec<usize>>,
        limit: Option<usize>,
        schema: SchemaRef,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(DbaseExec::new(
            projections,
            limit,
            schema,
            self.clone(),
        )))
    }
}

#[async_trait]
impl TableProvider for DbaseTable {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        let reader = self.reader.lock().unwrap();
        let dbase_fields = reader.fields();

        let arrow_fields: Vec<_> = dbase_fields
            .into_iter()
            .filter(|x| x.name() != "DeletionFlag")
            .map(|field| {
                let ftype = match field.field_type {
                    FieldType::Character => DataType::Utf8,
                    FieldType::Currency => DataType::Float64,
                    FieldType::Date => DataType::Date32, // days
                    FieldType::DateTime => DataType::Int64,
                    FieldType::Double => DataType::Float64,
                    FieldType::Float => DataType::Float32,
                    FieldType::Integer => DataType::Int32,
                    FieldType::Logical => DataType::Boolean,
                    FieldType::Memo => DataType::Utf8,
                    FieldType::Numeric => DataType::Float64,
                };
                Field::new(field.name().to_lowercase(), ftype, true)
            })
            .collect();

        SchemaRef::new(Schema::new(arrow_fields))
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        _state: &SessionState,
        projection: Option<&Vec<usize>>,
        // filters and limit can be used here to inject some push-down operations if needed
        _filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        return self
            .create_physical_plan(projection, limit, self.schema())
            .await;
    }
}

struct DbaseExec {
    table: DbaseTable,
    projected_schema: SchemaRef,
    projections: Vec<usize>,
    limit: usize,
}

impl DbaseExec {
    fn new(
        projections: Option<&Vec<usize>>,
        limit: Option<usize>,
        schema: SchemaRef,
        db: DbaseTable,
    ) -> Self {
        let projected_schema = project_schema(&schema, projections).unwrap();

        let projections = match projections {
            Some(proj) => proj.to_vec(),
            None => (0..schema.fields.len()).collect(),
        };

        let limit = limit.unwrap_or(db.num_records());

        Self {
            table: db,
            projected_schema,
            projections,
            limit,
        }
    }
}

impl Debug for DbaseExec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("custom_db")
    }
}

impl DisplayAs for DbaseExec {
    fn fmt_as(&self, t: DisplayFormatType, f: &mut Formatter<'_>) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "DbaseExec: {:?}", self.table.path)?;
            }
        }
        Ok(())
    }
}

impl ExecutionPlan for DbaseExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.projected_schema.clone()
    }

    fn output_partitioning(&self) -> datafusion::physical_plan::Partitioning {
        datafusion::physical_plan::Partitioning::UnknownPartitioning(1)
    }

    fn output_ordering(&self) -> Option<&[PhysicalSortExpr]> {
        None
    }

    fn children(&self) -> Vec<Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let schema = self.schema();
        let mut column_builders = Vec::<Box<dyn ArrayBuilder>>::new();
        let num_records = self.table.num_records();

        let schema_fields = schema.fields();

        for field in schema_fields {
            match field.data_type() {
                DataType::Utf8 => column_builders.push(Box::new(StringBuilder::with_capacity(
                    num_records,
                    num_records * 10,
                ))), // assume 10 chars per string
                DataType::Int32 => {
                    column_builders.push(Box::new(Int32Builder::with_capacity(num_records)))
                }
                DataType::Int64 => {
                    column_builders.push(Box::new(Int64Builder::with_capacity(num_records)))
                }
                DataType::Date32 => {
                    column_builders.push(Box::new(Date32Builder::with_capacity(num_records)))
                }
                DataType::Float64 => {
                    column_builders.push(Box::new(Float64Builder::with_capacity(num_records)))
                }
                DataType::Boolean => {
                    column_builders.push(Box::new(BooleanBuilder::with_capacity(num_records)))
                }
                _ => panic!("Unsupported field type"),
            };
        }

        let mut reader = self.table.reader.lock().unwrap();
        reader.seek(0).unwrap();

        let dbase_field_names: Vec<String> = reader
            .fields()
            .iter()
            .map(|x| x.name().to_owned())
            .collect();

        let records = reader.iter_records();

        for (l, record) in records.into_iter().enumerate() {
            if l >= self.limit {
                break;
            }

            let r = record.map_err(|e| DataFusionError::Execution(e.to_string()))?;

            for i in 0..self.projections.len() {
                match r.get(&dbase_field_names[self.projections[i]]).unwrap() {
                    FieldValue::Character(c) => match c {
                        Some(c) => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<StringBuilder>()
                            .unwrap()
                            .append_value(c.to_string()),
                        None => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<StringBuilder>()
                            .unwrap()
                            .append_null(),
                    },
                    FieldValue::Currency(f) => column_builders[i]
                        .as_any_mut()
                        .downcast_mut::<Float64Builder>()
                        .unwrap()
                        .append_value(*f),
                    FieldValue::Date(d) => match d {
                        Some(d) => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Date32Builder>()
                            .unwrap()
                            .append_value(d.to_unix_days()),

                        None => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Date32Builder>()
                            .unwrap()
                            .append_null(),
                    },
                    FieldValue::DateTime(d) => match d {
                        d => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Int64Builder>()
                            .unwrap()
                            .append_value(d.to_unix_timestamp()),
                    },
                    FieldValue::Double(d) => column_builders[i]
                        .as_any_mut()
                        .downcast_mut::<Float64Builder>()
                        .unwrap()
                        .append_value(*d),
                    FieldValue::Float(f) => match f {
                        Some(f) => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Float32Builder>()
                            .unwrap()
                            .append_value(*f),
                        None => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Float32Builder>()
                            .unwrap()
                            .append_null(),
                    },
                    FieldValue::Integer(v) => column_builders[i]
                        .as_any_mut()
                        .downcast_mut::<Int32Builder>()
                        .unwrap()
                        .append_value(*v),
                    FieldValue::Logical(l) => match l {
                        Some(l) => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<BooleanBuilder>()
                            .unwrap()
                            .append_value(*l),
                        None => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<BooleanBuilder>()
                            .unwrap()
                            .append_null(),
                    },
                    FieldValue::Memo(m) => column_builders[i]
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap()
                        .append_value(m.escape_unicode().to_string()),
                    FieldValue::Numeric(n) => match n {
                        Some(n) => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Float64Builder>()
                            .unwrap()
                            .append_value(*n),
                        None => column_builders[i]
                            .as_any_mut()
                            .downcast_mut::<Float64Builder>()
                            .unwrap()
                            .append_null(),
                    },
                }
            }
        }

        let array_refs: Vec<ArrayRef> = column_builders
            .iter_mut()
            .map(|builder| builder.finish())
            .collect();

        Ok(Box::pin(MemoryStream::try_new(
            vec![RecordBatch::try_new(
                self.projected_schema.clone(),
                array_refs,
            )?],
            self.schema(),
            None,
        )?))
    }

    fn statistics(&self) -> Statistics {
        Statistics::default()
    }
}

pub struct DbaseTableFactory {}

#[async_trait]
impl TableProviderFactory for DbaseTableFactory {
    async fn create(
        &self,
        _: &SessionState,
        cmd: &CreateExternalTable,
    ) -> Result<Arc<dyn TableProvider>> {
        let table = DbaseTable::new(&cmd.location);

        Ok(Arc::new(table))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use datafusion::arrow::array::StringArray;
    use datafusion::error::Result;
    use datafusion::execution::context::SessionState;
    use datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_simple_query() -> Result<()> {
        let cfg = RuntimeConfig::new();
        let env = RuntimeEnv::new(cfg).unwrap();
        let ses = SessionConfig::new();
        let mut state = SessionState::with_config_rt(ses, Arc::new(env));

        // add DbaseTableFactory to support "create external table stored as dbase" syntax
        state
            .table_factories_mut()
            .insert("DBASE".to_string(), Arc::new(DbaseTableFactory {}));

        let ctx = SessionContext::with_state(state);

        // register DBF file as external table
        let sql =
            "create external table stations stored as dbase location './tests/data/stations.dbf'";

        ctx.sql(sql).await?;

        // execute the query
        let df = ctx
            .sql(
                "
            select name
            from stations
            where line='blue'
            and name like 'F%'",
            )
            .await?;

        // expected result:
        // +-----------------------+
        // | name                  |
        // +-----------------------+
        // | Franconia-Springfield |
        // | Federal Center SW     |
        // +-----------------------+

        // extract first (and only) RecordBatch from dataframe
        let result = df.collect().await?;

        // ensure schema matches
        let expected_schema = Schema::new(vec![Field::new("name", DataType::Utf8, true)]);
        assert_eq!(result[0].schema(), Arc::new(expected_schema));

        // ensure values match
        assert_eq!(
            result[0].column(0).as_ref(),
            &StringArray::from(vec!["Franconia-Springfield", "Federal Center SW"])
        );
        Ok(())
    }
}
