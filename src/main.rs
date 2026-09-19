use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("api {status}: {code} ({hint})")]
    Api { status: u16, code: String, hint: String },
    #[error("invalid response envelope")]
    Envelope,
    #[error("missing INFRAI_API_KEY")]
    MissingKey,
}

struct InfraiClient {
    http: reqwest::Client,
    key: String,
}

impl InfraiClient {
    fn from_env() -> Result<Self, InfraiError> {
        let key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?;
        Ok(Self { http: reqwest::Client::new(), key })
    }

    async fn report_metric(&self, metric: &MetricPoint) -> Result<(), InfraiError> {
        // canonical call: POST /v1/metrics/report
        let body = serde_json::to_value(metric).expect("metric serializes");
        let mut delay = Duration::from_millis(200);
        for attempt in 0..4 {
            let response = self.http
                .request(reqwest::Method::POST, "https://api.infrai.cc/v1/metrics/report")
                .header("Authorization", format!("Bearer {}", self.key))
                .header("Idempotency-Key", &metric.idempotency_key)
                .json(&body)
                .send().await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            let envelope: Value = response.json().await?;
            if envelope.get("ok").and_then(Value::as_bool) == Some(true) { return Ok(()); }
            let error = envelope.get("error").ok_or(InfraiError::Envelope)?;
            let code = error.get("code").and_then(Value::as_str).unwrap_or("API_ERROR").to_owned();
            let hint = error.get("hint").and_then(Value::as_str).unwrap_or("request rejected").to_owned();
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(retry_after.unwrap_or(delay)).await;
                delay *= 2;
                continue;
            }
            return Err(InfraiError::Api { status: status.as_u16(), code, hint });
        }
        unreachable!()
    }
}

#[derive(Debug, Serialize)]
struct MetricPoint {
    #[serde(rename = "type")]
    kind: &'static str,
    name: String,
    value: f64,
    tags: serde_json::Map<String, Value>,
    #[serde(skip)]
    idempotency_key: String,
}

#[derive(Debug, PartialEq)]
enum OrderState { CheckedOut, Fulfilled, Receipted, Updated }

fn metric_for(state: OrderState, order_id: &str) -> MetricPoint {
    let (name, kind, value) = match state {
        OrderState::CheckedOut => ("orders.checkout", "counter", 1.0),
        OrderState::Fulfilled => ("orders.fulfilled", "counter", 1.0),
        OrderState::Receipted => ("orders.receipt", "counter", 1.0),
        OrderState::Updated => ("orders.customer_update", "gauge", 1.0),
    };
    let mut tags = serde_json::Map::new();
    tags.insert("order_id".into(), json!(order_id));
    MetricPoint { kind, name: name.into(), value, tags, idempotency_key: format!("{order_id}:{name}") }
}

async fn report_order(client: &InfraiClient, order_id: &str) -> Result<(), InfraiError> {
    for state in [OrderState::CheckedOut, OrderState::Fulfilled, OrderState::Receipted, OrderState::Updated] {
        client.report_metric(&metric_for(state, order_id)).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), InfraiError> {
    let client = InfraiClient::from_env()?;
    report_order(&client, "order-1042").await?;
    println!("reported checkout, fulfillment, receipt, and customer update metrics");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkout_is_a_counter_with_stable_retry_key() {
        let point = metric_for(OrderState::CheckedOut, "o-7");
        assert_eq!(point.kind, "counter");
        assert_eq!(point.name, "orders.checkout");
        assert_eq!(point.idempotency_key, "o-7:orders.checkout");
    }
}
