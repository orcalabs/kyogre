use futures::{Stream, TryStreamExt};
use kyogre_core::{Arrival, FiskeridirVesselId, PortDockPoint, TripId};

use crate::{
    error::Result,
    models::{Port, TripDockPoints, TripPorts},
    PostgresAdapter,
};

impl PostgresAdapter {
    pub(crate) async fn dock_points_impl(&self) -> Result<Vec<PortDockPoint>> {
        let docks = sqlx::query_as!(
            PortDockPoint,
            r#"
SELECT
    p.port_id,
    p.port_dock_point_id,
    p.latitude,
    p.longitude,
    p.name
FROM
    port_dock_points p
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(docks)
    }

    pub(crate) async fn dock_points_of_port_impl(
        &self,
        port_id: &str,
    ) -> Result<Vec<PortDockPoint>> {
        let docks = sqlx::query_as!(
            PortDockPoint,
            r#"
SELECT
    p.port_id,
    p.port_dock_point_id,
    p.latitude,
    p.longitude,
    p.name
FROM
    port_dock_points p
WHERE
    p.port_id = $1
            "#,
            port_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(docks)
    }

    pub(crate) async fn all_ers_arrivals_impl(&self) -> Result<Vec<Arrival>> {
        let arrivals = sqlx::query_as!(
            Arrival,
            r#"
SELECT
    fiskeridir_vessel_id AS "fiskeridir_vessel_id!: FiskeridirVesselId",
    arrival_timestamp AS "timestamp",
    port_id
FROM
    ers_arrivals
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(arrivals)
    }

    pub(crate) fn ports_impl(&self) -> impl Stream<Item = Result<Port>> + '_ {
        sqlx::query_as!(
            Port,
            r#"
SELECT
    p.port_id AS "id!",
    p.name,
    p.latitude,
    p.longitude
FROM
    ports AS p
            "#,
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
    }

    pub(crate) async fn port_impl(&self, port_id: &str) -> Result<Option<Port>> {
        let port = sqlx::query_as!(
            Port,
            r#"
SELECT
    p.port_id AS "id!",
    p.name,
    p.latitude,
    p.longitude
FROM
    ports AS p
WHERE
    p.port_id = $1
            "#,
            port_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(port)
    }

    pub async fn ports_of_trip_impl(&self, trip_id: TripId) -> Result<TripPorts> {
        let ports = sqlx::query_as!(
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
            trip_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ports)
    }

    pub async fn dock_points_of_trip_impl(&self, trip_id: TripId) -> Result<TripDockPoints> {
        let docks = sqlx::query_as!(
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
            trip_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(docks)
    }
}
