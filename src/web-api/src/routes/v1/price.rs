use actix_web::web;
use kyogre_core::PriceQuery;
use oasgen::oasgen;
use serde_qs::web::QsQuery as Query;

use crate::{Database, error::Result, response::Response};

#[oasgen(skip(db), tags("Price"))]
#[tracing::instrument(skip(db))]
pub async fn price<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<PriceQuery>,
) -> Result<Response<Option<f64>>> {
    let price = db.price(params.into_inner()).await?;
    Ok(Response::new(price))
}
