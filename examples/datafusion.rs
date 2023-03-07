use datafusion::error::Result;
use datafusion::execution::context::SessionState;
use datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use datafusion::prelude::*;
use dbase::{DbaseTable, DbaseTableFactory};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // create local execution context
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
    let sql = "create external table stations stored as dbase location './tests/data/stations.dbf'";
    ctx.sql(sql).await.unwrap();

    // execute the query
    let df = ctx
        .sql(
            "
        select *
        from stations
        where line='blue'
            and name like 'F%'",
        )
        .await?;

    df.show().await?;

    // alternatively, we can manually create and register the table
    let stations_table = DbaseTable::new("./tests/data/stations.dbf");
    ctx.register_table("stations2", Arc::new(stations_table))
        .expect("failed to register table");

    // self join as a simple example
    let df2 = ctx
        .sql(
            "
        select s1.name, s1.line as line_1, s2.line as line_2
        from stations s1
        join stations2 s2
            on s1.name = s2.name",
        )
        .await?;
    // print the results
    df2.show().await?;

    // can then call df.write_csv(), df.write_parquet(), etc.

    Ok(())
}
