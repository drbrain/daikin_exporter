Prometheus exporter for Daikin ComfortControl HVAC units

This prometheus exporter is for the Daikin BRP072A43 wifi adapter and similar
wifi-enabled Daikin units that are supported by the Daikin ComfortControl
application.

## Configuration

The below values are defaults (except `hosts`, it's commented out and provided
as an example).  All interval values are in milliseconds.

```toml
discover_interval = 300000
refresh_interval = 2000
refresh_timeout = 250
# hosts = ["192.0.2.10", "192.0.2.11"]
```

The `discover_interval` is the interval between discover broadcast requests.
The default is 5 minutes.

The `refresh_interval` is the interval in ms between unit refreshes.  The
default is 2 seconds.

The `refresh_timeout` is the time in ms to wait for a response before ignoring
the refresh attempt.  The default is 250 milliseconds.

`hosts` is the HVAC unit IP addresses (or hostnames).  By default the exporter
uses the Daikin UDP discovery protocol to discover hosts so this is not
necessary.  You will need to configure the HVAC adaptors to have static IP
addresses.

The BRP072A4X seems slow, so these values seems adequate to keep them from
frequent timeouts.

Values are cached between refreshes so if a unit times-out stale data will be
returned.

