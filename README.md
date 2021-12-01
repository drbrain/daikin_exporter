Prometheus exporter for Daikin ComfortControl HVAC units

This prometheus exporter is for the Daikin BRP072A43 wifi adapter and similar
wifi-enabled Daikin units that are supported by the Daikin ComfortControl
application.

## Configuration

You may provide a toml-format configuration file as the first argument to
`daikin_exporter`.  The only argument you will probably need to set is the
`refresh_interval` which should be about half the prometheus `scrape_interval`.

The below values are defaults (except `hosts`, it's commented out and provided
as an example).  All interval values are in milliseconds.

```toml
discover_major_interval = 300000
discover_minor_interval = 200
refresh_interval = 7500
refresh_timeout = 250
```

The `discover_major_interval` is the long interval between discover broadcast
requests.  The default is 5 minutes.

The `discover_minor_interval` is the short interval between discover broadcast
requests.  The default is 200 milliseconds.

The ComfortControl iOS app sends two requests about 200 milliseconds apart,
then repeats the broadcast about 3 seconds later.  To avoid excessive UDP
traffic the exporter is much more conservative for the major interval.

The `refresh_interval` is the interval in ms between unit refreshes and should
be half the prometheus `scrape_interval`.  The default is 7.5 seconds.

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

