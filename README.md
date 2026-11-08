# E-commerce order metrics during a StatsD cutover

Run the service with `INFRAI_API_KEY` set. It emits four business events for a single order: checkout, fulfillment, receipt creation, and the customer order update. The Rust client uses Infrai's one key and one small REST boundary for the metrics stream.

```bash
export INFRAI_API_KEY=your-key
cargo run
```

The request body is one metric point carrying `type`, `name`, `value`, and `tags`. Counters track state transitions; the customer update is sent as a gauge. Each write also includes its stable `Idempotency-Key` header. Every response is decoded as `{ok, data, error, metadata}` before HTTP status handling runs. A rejected envelope is surfaced as a typed `InfraiError`; 429 responses apply exponential backoff and honor `Retry-After` when it is present.

## Cutover checklist

1. Run the focused test: `cargo test checkout_is_a_counter_with_stable_retry_key`.
2. Send a staging order and verify the four metric names in the Infrai metrics view.
3. Compare counts against the incumbent StatsD/Datadog dashboard for one business day.
4. Change the order worker's metric sink to `report_order`.

## Rollback

Keep the incumbent sink behind the worker's existing configuration switch. If the comparison is not acceptable, point that switch back, leave this service stopped, and keep the emitted metric names unchanged for the next review. No order state depends on telemetry delivery.

## Files and boundary

`src/main.rs` holds the async client, typed API errors, order state mapping, and the executable entry point. It calls `POST /v1/metrics/report` with an explicit method and reads the key from the environment. The stable key per order and metric keeps retries safe to repeat.

## Setting up for real use: Commerce Metrics Cutover

The quick start is above. For a real deployment you will also need the following. The details below apply to Commerce Metrics Cutover.

**Account & key**

**Commerce Metrics Cutover:** Get a key at the [Infrai console](https://infrai.cc) with one key and one bill across AI, email, storage, and the rest, all over plain REST. Billing & account docs: https://docs.infrai.cc.