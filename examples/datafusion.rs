use datafusion::error::Result;

use datafusion::execution::SessionStateBuilder;
use datafusion::prelude::*;
use dbase::{DbaseDataSource, DbaseTableFactory};
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // create local execution context
    let table_factory = Arc::new(DbaseTableFactory {});
    let session_state = SessionStateBuilder::new()
        .with_default_features()
        .with_table_factory("DBASE".to_string(), table_factory)
        .build();

    // add DbaseTableFactory to support "create external table stored as dbase" syntax
    let ctx = SessionContext::new_with_state(session_state);

    // register DBF file as external table
    let sql = "create external table stations stored as dbase location './tests/data/stations.dbf'";
    ctx.sql(sql).await.unwrap();

    // execute the query
    let df = ctx
        .sql(
            "
            select
                name, line, `marker-col`
            from
                stations
            where
                line='blue'
                and name like 'F%'
        ",
        )
        .await?;

    df.show().await?;

    // alternatively, we can manually create and register the table
    let stations_table = DbaseDataSource::new("./tests/data/stations.dbf");
    ctx.register_table("stations2", Arc::new(stations_table))
        .expect("failed to register table");

    // self join as a simple example
    let df2 = ctx
        .sql(
            "
            select 
                s1.name, 
                s1.line as line_1, 
                s2.line as line_2
            from 
                stations s1
                join stations2 s2
                    on s1.name = s2.name
            limit 10;
            ",
        )
        .await?;
    // print the results
    df2.show().await?;

    // can then call df.write_csv(), df.write_parquet(), etc.

    Ok(())
}
