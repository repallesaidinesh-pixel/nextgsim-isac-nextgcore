# ISAC Uplink Reporting Integration

## Status and scope

This document records the experimental Integrated Sensing and
Communication (ISAC) reporting path implemented and tested in this
NextGSim working tree.

The implementation connects simulated UE-side ranging measurements to
the gNB ISAC task through an uplink RRC measurement-report path. The
application payload is an experimental format carried using the
RRC `MeasurementReportAppLayer-r17` message extension.

This is a simulator integration, not evidence of physical radio sensing
or USRP hardware validation.

## Data path

1. The UE-side ISAC sensor produces a simulated ranging report.
2. The report is encoded in the experimental application payload.
3. The payload is carried in an uplink RRC
   `MeasurementReportAppLayer-r17` message.
4. The gNB RRC dispatch code recognizes and decodes the report.
5. The gNB forwards the decoded report through the shared ISAC channel.
6. The gNB ISAC task logs the reporting UE, peer UE, distance, accuracy,
   measurement count, and measurement method.

The payload implementation and the RRC reporting/dispatch implementation
are maintained in:

- `nextgsim-rrc/src/procedures/isac_payload.rs`
- `nextgsim-rrc/src/procedures/isac_reporting.rs`
- `nextgsim-rrc/src/procedures/dcch_dispatch.rs`

The gNB task and startup wiring are maintained in the `nextgsim-gnb`
crate.

## Experimental payload fields

The current application payload includes:

- A version field.
- Peer UE identifier.
- Distance in millimetres.
- Accuracy estimate in millimetres.
- Measurement count.
- Measurement-method code.

This is an experimental application-layer encoding. It should not be
interpreted as a standardized 3GPP ISAC payload format merely because
the containing RRC message uses a standardized message extension.

## gNB ISAC and NWDAF configuration

The host configuration used during the verified simulator run enables
both ISAC and NWDAF:

```yaml
isac_enabled: true
nwdaf_enabled: true
```

Use the project's existing configuration documentation and a
machine-specific local configuration to supply the remaining network
settings. Do not publish subscriber credentials, private keys, or
unredacted local deployment configuration.

## Verified simulator observations

During the verified run, the gNB logged reports similar to:

```text
ISAC report: reporting UE 0, peer UE 2, distance=5.000 m,
accuracy=0.010 m, measurements=131680, method=carrier_phase
```

The values above are simulator-reported outputs. In particular,
`accuracy=0.010 m` is an estimated accuracy field produced by the
simulator; it is not an independently measured physical error bound.

With NWDAF enabled, the gNB also started the NWDAF task and emitted
periodic cell-load reports. The observed occupancy was traffic-derived,
not a direct physical-layer PRB utilization measurement.

## Regression tests recorded for this implementation

The following test commands completed successfully in the development
environment:

```bash
cargo test -p nextgsim-rrc
cargo test -p nextgsim-gnb --features "nextgsim-isac,nextgsim-nwdaf"
cargo test -p nextgsim-ue
cargo test -p nextgsim-integration-tests --features 6g --test sixg_task_integration
cargo test -p nextgsim-integration-tests --features sidelink --test ranging_report_e2e
cargo fmt --all -- --check
git diff --check
```

The latest reported regression subset comprised 980 passing tests with
zero failures across RRC, gNB, 6G task integration, and ranging E2E.
The UE test suite was also run separately and passed. Test counts can
change as the codebase evolves; rerun the commands above when reproducing
the result.

## Known limitations

- The range and accuracy values are simulator outputs; physical accuracy
  has not been independently validated.
- This integration does not establish that the experimental payload is
  a standardized 3GPP ISAC application payload.
- NWDAF occupancy in the observed logs is traffic-derived rather than a
  direct PRB measurement.
- USRP or other SDR hardware validation has not been performed as part
  of this software verification.
- Deployment configuration is environment-specific and should be
  maintained separately from public example documentation.

## Reproducing the host run

From the repository root, build the gNB with the required features:

```bash
cargo build --release -p nextgsim-gnb --features "nextgsim-isac,nextgsim-nwdaf"
```

Then run it with a locally configured host YAML file:

```bash
RUST_LOG=info,nextgsim_gnb=debug \
./target/release/nr-gnb -c config/gnb-isac-host.yaml
```

The configuration filename above refers to the local development
configuration used in the recorded run. Ensure that its network
addresses and other settings match your environment before starting
the simulator.
