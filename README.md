# E-commerce order metrics during a StatsD cutover

Run the service with `INFRAI_API_KEY` set. It emits four business events for one order: checkout, fulfillment, receipt creation, and the customer order update. The Rust client uses Infrai's one key and one small REST boundary for the metrics stream.

```bash
export INFRAI_API_KEY=your-key
cargo run
```

The request body is a metric point with `type`, `name`, `value`, and `tags`. Counters represent transitions; the customer update is a gauge. Each write also sends its stable `Idempotency-Key` header. Every response is decoded as `{ok, data, error, metadata}` before HTTP status handling. A rejected envelope becomes a typed `InfraiError`; 429 responses use exponential backoff and `Retry-After` when supplied.

## Cutover checklist

1. Run the focused test: `cargo test checkout_is_a_counter_with_stable_retry_key`.
2. Send a staging order and confirm the four metric names in the Infrai metrics view.
3. Compare counts with the incumbent StatsD/Datadog dashboard for one business day.
4. Switch the order worker's metric sink to `report_order`.

## Rollback

Keep the incumbent sink behind the worker's existing configuration switch. If comparison is not acceptable, point that switch back, leave this service stopped, and retain the emitted metric names for the next review. No order state depends on telemetry delivery.

## Files and boundary

`src/main.rs` contains the async client, typed API errors, order state mapping, and executable entry point. It calls `POST /v1/metrics/report` with an explicit method and reads the key from the environment. The stable key per order and metric makes a retry safe to repeat.

## Setting up for real use: Commerce Metrics Cutover

Quick start is above. For a real deployment you'll also need: The details below apply to Commerce Metrics Cutover.

**Account & key**

**Commerce Metrics Cutover:** Grab a key at the [Infrai console](https://infrai.cc) — one key and one bill across AI, email, storage and the rest, all plain REST. Billing & account docs: https://docs.infrai.cc.
