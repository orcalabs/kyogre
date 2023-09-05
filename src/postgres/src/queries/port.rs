use crate::{
    error::PostgresError,
    models::{Arrival, NewPort, TripDockPoints, TripPorts},
    PostgresAdapter,
};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::TripId;
use unnest_insert::UnnestInsert;

impl PostgresAdapter {
    pub(crate) async fn all_ers_arrivals_impl(&self) -> Result<Vec<Arrival>, PostgresError> {
        sqlx::query_as!(
            Arrival,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!",
    arrival_timestamp AS "timestamp",
    port_id
FROM
    ers_arrivals
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
    pub(crate) async fn add_ports<'a>(
        &'a self,
        ports: Vec<NewPort>,
        tx: &mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Result<(), PostgresError> {
        NewPort::unnest_insert(ports, &mut **tx)
            .await
            .into_report()
            .change_context(PostgresError::Query)
            .map(|_| ())
    }

    pub async fn ports_of_trip_impl(&self, trip_id: TripId) -> Result<TripPorts, PostgresError> {
        sqlx::query_as!(
            TripPorts,
            r#"
SELECT
    p.port_id AS "start_port_id?",
    p.name AS start_port_name,
    p.nationality AS "start_port_nationality?",
    p.latitude AS start_port_latitude,
    p.longitude AS start_port_longitude,
    e.end_port_id AS "end_port_id?",
    e.end_port_name,
    e.end_port_nationality AS "end_port_nationality?",
    e.end_port_latitude,
    e.end_port_longitude
FROM
    trips AS t
    LEFT JOIN ports AS p ON t.start_port_id = p.port_id
    LEFT JOIN (
        SELECT
            t2.trip_id,
            p2.port_id AS end_port_id,
            p2.name AS end_port_name,
            p2.nationality AS end_port_nationality,
            p2.latitude AS end_port_latitude,
            p2.longitude AS end_port_longitude
        FROM
            trips AS t2
            LEFT JOIN ports AS p2 ON t2.end_port_id = p2.port_id
        WHERE
            t2.trip_id = $1
    ) AS e ON e.trip_id = t.trip_id
WHERE
    t.trip_id = $1
            "#,
            trip_id.0,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }

    pub async fn dock_points_of_trip_impl(
        &self,
        trip_id: TripId,
    ) -> Result<TripDockPoints, PostgresError> {
        sqlx::query_as!(
            TripDockPoints,
            r#"
WITH
    q1 AS (
        SELECT
            JSONB_AGG(
                JSON_BUILD_OBJECT(
                    'port_id',
                    pd.port_id,
                    'port_dock_point_id',
                    pd.port_dock_point_id,
                    'latitude',
                    pd.latitude,
                    'longitude',
                    pd.longitude,
                    'name',
                    pd.name
                )
            ) AS "start"
        FROM
            trips AS t
            INNER JOIN ports AS p ON t.start_port_id = p.port_id
            INNER JOIN port_dock_points AS pd ON pd.port_id = p.port_id
        WHERE
            trip_id = $1
    ),
    q2 AS (
        SELECT
            JSONB_AGG(
                JSON_BUILD_OBJECT(
                    'port_id',
                    pd.port_id,
                    'port_dock_point_id',
                    pd.port_dock_point_id,
                    'latitude',
                    pd.latitude,
                    'longitude',
                    pd.longitude,
                    'name',
                    pd.name
                )
            ) AS "end"
        FROM
            trips AS t
            INNER JOIN ports AS p ON t.end_port_id = p.port_id
            INNER JOIN port_dock_points AS pd ON pd.port_id = p.port_id
        WHERE
            trip_id = $1
    )
SELECT
    "start"::TEXT,
    "end"::TEXT
FROM
    q1
    CROSS JOIN q2;
            "#,
            trip_id.0,
        )
        .fetch_one(&self.pool)
        .await
        .into_report()
        .change_context(PostgresError::Query)
    }
}
