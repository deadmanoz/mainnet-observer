use crate::gen_csv::PROXY_POOL_GROUP_ANTPOOL;
use crate::schema;
use crate::stats::{
    BlockStats, FeerateStats, InputStats, OutputStats, ScriptStats, Stats, TxStats,
};
use crate::MainError;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Float, Integer, Text};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::{debug, info};
use std::collections::BTreeSet;
use std::error::Error;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub type MigrationError = Box<dyn Error + Send + Sync>;

#[derive(Debug, QueryableByName)]
pub struct TableInfo {
    #[diesel(sql_type = Text)]
    pub name: String,
}

#[derive(Debug, QueryableByName)]
pub struct AvgAndSum {
    #[diesel(sql_type = Float)]
    pub avg: f32,
    #[diesel(sql_type = BigInt)]
    pub sum: i64,
}

#[derive(Debug, QueryableByName)]
pub struct DateColumn {
    #[diesel(sql_type = Text)]
    pub date: String,
}

pub fn open_db_and_run_migrations(database_path: &str) -> Result<SqliteConnection, MainError> {
    debug!("trying to open database: {}", database_path);
    let mut conn = SqliteConnection::establish(&database_path)?;
    debug!("trying to run pending migrations..");
    conn.run_pending_migrations(MIGRATIONS)?;
    info!("database {} opened", database_path);
    return Ok(conn);
}

pub fn performance_tune(conn: &mut SqliteConnection) -> Result<(), diesel::result::Error> {
    debug!("performance tuning the database for batch inserts..");
    sql_query(
        r#"
        pragma journal_mode = WAL;
        pragma synchronous = normal;
        pragma temp_store = memory;
    "#,
    )
    .execute(conn)?;
    Ok(())
}

pub fn get_db_block_height(
    conn: &mut SqliteConnection,
) -> Result<Option<i64>, diesel::result::Error> {
    return schema::block_stats::dsl::block_stats
        .select(diesel::dsl::max(schema::block_stats::height))
        .first(conn);
}

pub fn list_column_names(
    conn: &mut SqliteConnection,
    table: &str,
) -> Result<Vec<TableInfo>, diesel::result::Error> {
    sql_query(format!("PRAGMA table_info({})", table)).get_results(conn)
}

pub fn column_sum_and_avg_by_date(
    conn: &mut SqliteConnection,
    colname: &str,
    table: &str,
) -> Vec<AvgAndSum> {
    sql_query(format!(
        "SELECT avg({}) as avg, sum({}) as sum FROM {} GROUP BY date",
        colname, colname, table
    ))
    .get_results(conn)
    .unwrap()
}

pub fn date_column(conn: &mut SqliteConnection) -> Vec<DateColumn> {
    sql_query(format!(
        "SELECT date as date FROM block_stats GROUP BY date"
    ))
    .get_results(conn)
    .unwrap()
}

#[derive(Debug, QueryableByName)]
pub struct MiningPoolID {
    #[diesel(sql_type = Integer)]
    pub pool_id: i32,
    #[diesel(sql_type = Integer)]
    pub count: i32,
}

pub fn current_top_mining_pools(
    conn: &mut SqliteConnection,
) -> Result<Vec<MiningPoolID>, diesel::result::Error> {
    sql_query(format!(
        r#"
        WITH recent_blocks AS (
            SELECT pool_id
            FROM block_stats
            ORDER BY height DESC
            LIMIT 2016*2
        )
        SELECT pool_id, COUNT(*) as count
        FROM recent_blocks
        GROUP BY pool_id
        HAVING COUNT(*) > 0
        ORDER BY count DESC;
        "#
    ))
    .get_results(conn)
}

#[derive(Debug, QueryableByName)]
pub struct Top5PoolBlocksPerDay {
    #[diesel(sql_type = Text)]
    pub date: String,
    #[diesel(sql_type = Integer)]
    pub top1_blocks: i32,
    #[diesel(sql_type = Integer)]
    pub top2_blocks: i32,
    #[diesel(sql_type = Integer)]
    pub top3_blocks: i32,
    #[diesel(sql_type = Integer)]
    pub top4_blocks: i32,
    #[diesel(sql_type = Integer)]
    pub top5_blocks: i32,
    #[diesel(sql_type = Integer)]
    pub total: i32,
}

// formats a vector of i32's to comma separated list of ids
// suitable for SQL
// vec![1, 2, 3] -> "1, 2, 3"
fn vec_to_string(ids: &[i32]) -> String {
    ids.iter()
        .map(|&num| num.to_string())
        .collect::<Vec<String>>()
        .join(", ")
}

/// Gets the blocks per day for the top 5 pool groups.
/// A pool group can either be single pool or a group of pools
/// like e.g. a proxy pool group.
pub fn blocks_per_day_top5_pool_groups(
    conn: &mut SqliteConnection,
    pool_groups: &[Vec<i32>; 5],
) -> Result<Vec<Top5PoolBlocksPerDay>, diesel::result::Error> {
    let mut all_ids = BTreeSet::new();
    for group in pool_groups.iter() {
        for id in group.iter() {
            all_ids.insert(*id);
        }
    }

    sql_query(format!(
        r#"
        SELECT * FROM (
            SELECT
                t."date",
                COUNT(CASE WHEN pool_id IN ({}) THEN 1 END) AS top1_blocks,
                COUNT(CASE WHEN pool_id IN ({}) then 1 END) AS top2_blocks,
                COUNT(CASE WHEN pool_id IN ({}) THEN 1 END) AS top3_blocks,
                COUNT(CASE WHEN pool_id IN ({}) THEN 1 END) AS top4_blocks,
                COUNT(CASE WHEN pool_id IN ({}) THEN 1 END) AS top5_blocks,
                COALESCE(subquery.total_count, 0) AS total
            FROM block_stats t
            LEFT JOIN (
                SELECT "date", COUNT(*) AS total_count
                FROM block_stats
                GROUP BY "date"
            ) subquery
                ON t."date" = subquery."date"
            WHERE pool_id IN ({})
            GROUP BY t."date", subquery.total_count
            ORDER BY t."date" DESC
            LIMIT (356 * 6)
        ) X
        ORDER BY "date" ASC;
        "#,
        // ids for CASE WHEN pool_id
        vec_to_string(&pool_groups[0]),
        vec_to_string(&pool_groups[1]),
        vec_to_string(&pool_groups[2]),
        vec_to_string(&pool_groups[3]),
        vec_to_string(&pool_groups[4]),
        // ids for WHERE pool_id IN
        vec_to_string(&all_ids.iter().map(|i| *i).collect::<Vec<i32>>()),
    ))
    .get_results(conn)
}

#[derive(Debug, QueryableByName)]
pub struct CentralizationIndex {
    #[diesel(sql_type = Text)]
    pub date: String,
    #[diesel(sql_type = Integer)]
    pub top1_count: i32,
    #[diesel(sql_type = Integer)]
    pub top2_count: i32,
    #[diesel(sql_type = Integer)]
    pub top3_count: i32,
    #[diesel(sql_type = Integer)]
    pub top4_count: i32,
    #[diesel(sql_type = Integer)]
    pub top5_count: i32,
    #[diesel(sql_type = Integer)]
    pub top6_count: i32,
    #[diesel(sql_type = Integer)]
    pub total_blocks: i32,
}

pub fn mining_centralization_index(
    conn: &mut SqliteConnection,
) -> Result<Vec<CentralizationIndex>, diesel::result::Error> {
    sql_query(
        r#"
        WITH RankedPoolCounts AS (
            SELECT
                date,
                pool_id,
                COUNT(*) AS pool_count,
                ROW_NUMBER() OVER (PARTITION BY date ORDER BY COUNT(*) DESC) AS rank
            FROM block_stats
            GROUP BY date, pool_id
        ),
        TotalBlocks AS (
            SELECT
            date,
            COUNT(*) AS total_blocks
            FROM block_stats
            GROUP BY date
        )
        SELECT
            r.date,
            SUM(CASE WHEN r.rank = 1 THEN r.pool_count ELSE 0 END) AS top1_count,
            SUM(CASE WHEN r.rank = 2 THEN r.pool_count ELSE 0 END) AS top2_count,
            SUM(CASE WHEN r.rank = 3 THEN r.pool_count ELSE 0 END) AS top3_count,
            SUM(CASE WHEN r.rank = 4 THEN r.pool_count ELSE 0 END) AS top4_count,
            SUM(CASE WHEN r.rank = 5 THEN r.pool_count ELSE 0 END) AS top5_count,
            SUM(CASE WHEN r.rank = 6 THEN r.pool_count ELSE 0 END) AS top6_count,
            t.total_blocks
        FROM RankedPoolCounts r
        JOIN TotalBlocks t ON r.date = t.date
        WHERE rank <= 6
        GROUP BY r.date, t.total_blocks
        ORDER BY r.date;
        "#,
    )
    .get_results(conn)
}

pub fn mining_centralization_index_with_proxy_pools(
    conn: &mut SqliteConnection,
) -> Result<Vec<CentralizationIndex>, diesel::result::Error> {
    sql_query(format!(
        r#"
        WITH RankedPoolCounts AS (
            SELECT
                date,
                CASE
                    WHEN pool_id IN ({}) THEN 9999  -- group "AntPool & friends" into pool 9999
                    ELSE pool_id  -- Keep other pools as they are
                END AS pool_group,
                COUNT(*) AS pool_count,
                ROW_NUMBER() OVER (PARTITION BY date ORDER BY COUNT(*) DESC) AS rank
            FROM block_stats
            GROUP BY date, pool_group
        ),
        TotalBlocks AS (
            SELECT
            date,
            COUNT(*) AS total_blocks
            FROM block_stats
            GROUP BY date
        )
        SELECT
            r.date,
            SUM(CASE WHEN r.rank = 1 THEN r.pool_count ELSE 0 END) AS top1_count,
            SUM(CASE WHEN r.rank = 2 THEN r.pool_count ELSE 0 END) AS top2_count,
            SUM(CASE WHEN r.rank = 3 THEN r.pool_count ELSE 0 END) AS top3_count,
            SUM(CASE WHEN r.rank = 4 THEN r.pool_count ELSE 0 END) AS top4_count,
            SUM(CASE WHEN r.rank = 5 THEN r.pool_count ELSE 0 END) AS top5_count,
            SUM(CASE WHEN r.rank = 6 THEN r.pool_count ELSE 0 END) AS top6_count,
            t.total_blocks
        FROM RankedPoolCounts r
        JOIN TotalBlocks t ON r.date = t.date
        WHERE rank <= 6
        GROUP BY r.date, t.total_blocks
        ORDER BY r.date;
        "#,
        vec_to_string(
            &(PROXY_POOL_GROUP_ANTPOOL
                .iter()
                .map(|i| *i as i32)
                .collect::<Vec<i32>>())
        ),
    ))
    .get_results(conn)
}

#[derive(QueryableByName)]
pub struct PoolBlockPerDay {
    #[diesel(sql_type = Text)]
    pub date: String,
    #[diesel(sql_type = BigInt)]
    pub count: i64,
    #[diesel(sql_type = BigInt)]
    pub total: i64,
}

pub fn get_blocks_per_day_per_pool(
    conn: &mut SqliteConnection,
    id: i32,
) -> Result<Vec<PoolBlockPerDay>, diesel::result::Error> {
    sql_query(format!(
        r#"
        SELECT
            b.date,
            count(*) AS count,
            t.total
        FROM
            block_stats b
        JOIN (
            SELECT
                date,
                count(*) AS total
            FROM
                block_stats
            GROUP BY
                date
        ) t ON b.date = t.date
        WHERE
            b."pool_id" = {}
        GROUP BY
            b.date, t.total;
        "#,
        id
    ))
    .get_results(conn)
}

pub fn insert_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<Stats>,
) -> Result<(), diesel::result::Error> {
    insert_block_stats(conn, &stats.iter().map(|s| s.block.clone()).collect())?;
    insert_tx_stats(conn, &stats.iter().map(|s| s.tx.clone()).collect())?;
    insert_input_stats(conn, &stats.iter().map(|s| s.input.clone()).collect())?;
    insert_output_stats(conn, &stats.iter().map(|s| s.output.clone()).collect())?;
    insert_script_stats(conn, &stats.iter().map(|s| s.script.clone()).collect())?;
    insert_feerate_stats(conn, &stats.iter().map(|s| s.feerate.clone()).collect())?;
    Ok(())
}

fn insert_block_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<BlockStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::block_stats;
    debug!("Inserting a batch of {} block stats", stats.len());

    if let Err(e) = diesel::insert_into(block_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} block stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(block_stats::table)
                            .values(stat)
                            .on_conflict(block_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}

fn insert_tx_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<TxStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::tx_stats;
    debug!("Inserting a batch of {} tx stats", stats.len());

    if let Err(e) = diesel::insert_into(tx_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} tx stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(tx_stats::table)
                            .values(stat)
                            .on_conflict(tx_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}

fn insert_input_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<InputStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::input_stats;
    debug!("Inserting a batch of {} input stats", stats.len());

    if let Err(e) = diesel::insert_into(input_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} input stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(input_stats::table)
                            .values(stat)
                            .on_conflict(input_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}

fn insert_output_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<OutputStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::output_stats;
    debug!("Inserting a batch of {} output stats", stats.len());

    if let Err(e) = diesel::insert_into(output_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} output stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(output_stats::table)
                            .values(stat)
                            .on_conflict(output_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}

fn insert_script_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<ScriptStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::script_stats;
    debug!("Inserting a batch of {} script stats", stats.len());

    if let Err(e) = diesel::insert_into(script_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} script stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(script_stats::table)
                            .values(stat)
                            .on_conflict(script_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}

fn insert_feerate_stats(
    conn: &mut SqliteConnection,
    stats: &Vec<FeerateStats>,
) -> Result<(), diesel::result::Error> {
    use crate::schema::feerate_stats;
    debug!("Inserting a batch of {} feerate stats", stats.len());

    if let Err(e) = diesel::insert_into(feerate_stats::table)
        .values(stats)
        .execute(conn)
    {
        match e {
            diesel::result::Error::DatabaseError(db_error, _) => match db_error {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    debug!(
                        "Falling back to individually inserting {} feerate stats: {}",
                        stats.len(),
                        e
                    );
                    for stat in stats.iter() {
                        diesel::insert_into(feerate_stats::table)
                            .values(stat)
                            .on_conflict(feerate_stats::height)
                            .do_update()
                            .set(stat)
                            .execute(conn)?;
                    }
                    return Ok(());
                }
                _ => return Err(e),
            },
            _ => return Err(e),
        }
    }
    Ok(())
}
