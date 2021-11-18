Prometheus exporter for Daikin ComfortControl HVAC units

This prometheus exporter is for the Daikin BRP072A43 wifi adapter and similar
wifi-enabled Daikin units that are supported by the Daikin ComfortControl
application.

## Configuration

```toml
interval = 5000
timeout = 1000
hosts = ["192.0.2.10", "192.0.2.11"]
```

`interval` is interval in ms between unit refreshes, `timeout` is the time in
ms to wait for a response, `hosts` is the HVAC unit IP addresses (or
hostnames).

The BRP072A43 is quite slow, so these values seems adequate to keep them from
frequent timeouts.

Values are cached between refreshes so if a unit times-out stale data will be
returned.

