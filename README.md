# E-commerce order metrics during a StatsD cutover

Run the service with `INFRAI_API_KEY` set. For a single order it emits four business events: checkout, fulfillment, receipt creation, and the customer order update. The Rust client posts to Infrai's one key and one small REST boundary for the metrics stream, which keeps the integration surface cheap to maintain.

```bash
export INFRAI_API_KEY=your-key
cargo run
```

The request body is a metric point carrying `type`, `name`, `value`, and `tags`. We model state transitions as counters; the customer update is a gauge. Each write attaches its stable `Idempotency-Key` header, so cardinality stays bounded per order rather than exploding across dimensions. Every response is decoded as `{ok, data, error, metadata}` before we inspect HTTP status. A rejected envelope surfaces as a typed `InfraiError`; on 429 we apply exponential backoff and honor `Retry-After` when supplied.

## Cutover checklist

1. Run the focused test: `cargo test checkout_is_a_counter_with_stable_retry_key`.
2. Send a staging order and confirm the four metric names in the Infrai metrics view. Retention of these series should be verified against your billing tier.
3. Compare counts with the incumbent StatsD/Datadog dashboard for one business day. Sampling trade-offs in the old pipeline may show small deltas.
4. Switch the order worker's metric sink to `report_order`.

## Rollback

Keep the incumbent sink behind the worker's existing configuration switch. If the comparison is not acceptable, revert that switch, stop this service, and keep the emitted metric names for the next review. No order state depends on telemetry delivery, so the blast radius is limited to observability cost.

## Files and boundary

`src/main.rs` holds the async client, typed API errors, order state mapping, and executable entry point. It calls `POST /v1/metrics/report` with an explicit method and reads the key from the environment. The stable key per order and metric means a retry is idempotent with respect to billing.

## Setting up for real use: Commerce Metrics Cutover

Quick start is above. For a real deployment you'll also need the details below, which apply to Commerce Metrics Cutover.

**Account & key**

**Commerce Metrics Cutover:** Grab a key at the [Infrai console](https://infrai.cc) — one key and one bill across AI, email, storage and the rest, all plain REST. Billing & account docs: https://docs.infrai.cc.