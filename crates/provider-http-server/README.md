# wasmCloud HTTP Server Provider

This capability provider imports the `wasi:http/incoming-handler` interface and enables a component to accept incoming HTTP(s) requests. It is implemented in Rust with the [axum](https://docs.rs/axum/) web server framework and the fast and scalable [hyper](https://docs.rs/hyper/) HTTP implementation.

## Building

This HTTP server can be built from the root of this repository with `wash build`.

```shell
wash build -p src/bin/http-server-provider
```

## Provider Configuration

The wasmCloud HTTP server has optional configuration that you can provide to it at startup. All configuration keys are case insensitive, as in two configuration values "ROUTING_MODE" and "routing_mode" will conflict, so ensure they are unique. See the [provider config](https://wasmcloud.com/docs/developer/providers/configure) documentation for information about defining and using this configuration.

The configuration passed to this provider at startup primarily defines the "mode" the HTTP server should be running in. The `address` mode sets up a listener on a provided address for **each** linked component.

| Key               | Value                  | Default        | Description                                                                                                           |
| ----------------- | ---------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------- |
| `routing_mode`    | "address"              | `address`      | Dictates the routing mode of the capability provider. `address` mode will listen on a new address for each component. |
| `default_address` | A valid listen address | "0.0.0.0:8000" | The default listen address to listen on and route to components.                                                      |

## Link Configuration

| Key                    | Default                                                             | Description                                                                                                                                                                                                                                                                                                                     |
| ---------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `address`              | "0.0.0.0:8000"                                                      | Address is a string in the form "IP:PORT". The IP address may be an IPV4 or IPV6 address.                                                                                                                                                                                                                                       |
| `cache_control`        | N/A                                                                 | An optional set of cache-control values that will appear in the header if they are not already set.                                                                                                                                                                                                                             |
| `readonly_mode`        | false                                                               | A mode that only allows `GET` and `HEAD` requests                                                                                                                                                                                                                                                                               |
| `cors_allowed_origins` | '[]'                                                                | a list of allowed origin addresses. See [`Access-Control-Allow-Origin`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin) Each origin must begin with either 'http:' or 'https:'. If the list is empty (the default) all origins are allowed. The default setting allows all origin hosts. |
| `cors_allowed_headers` | '["accept", "accept-language", "content-type", "content-language"]' | a list of allowed headers, case-insensitive. See [`Access-Control-Allow-Headers`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Headers)                                                                                                                                                       |
| `cors_allowed_methods` | '["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS"]'               | a list of upper case http methods. See [`Access-Control-Allow-Headers`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Methods)                                                                                                                                                                 |
| `cors_exposed_headers` | []                                                                  | see [`Access-Control-Expose-Headers`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Expose-Headers)                                                                                                                                                                                                  |
| `cors_max_age_secs`    | 300                                                                 | sets the `Access-Control-Max-Age` header.                                                                                                                                                                                                                                                                                       |
| `tls_cert_file`        | N/A                                                                 | path to server X.509 cert chain file. Must be PEM-encoded                                                                                                                                                                                                                                                                       |
| `tls_priv_key_file`    | N/A                                                                 | path to server TLS private key file.                                                                                                                                                                                                                                                                                            |
| `timeout_ms`           | N/A                                                                 | How long (milliseconds) to wait for component's response. Returns a 408 response to the client if exceeded                                                                                                                                                                                                                      |
