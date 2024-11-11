# chbot 

Simple Discord bot to query [chproxy](https://www.chproxy.org/).

```
Usage: chbot [OPTIONS] --user <USER> --password <PASSWORD> --token <TOKEN>

Options:
      --url <URL>                    [default: https://clickhouse.nxthdr.dev]
  -u, --user <USER>                  ClickHouse user
  -p, --password <PASSWORD>          ClickHouse password
  -t, --token <TOKEN>                Discord bot token
      --output-limit <OUTPUT_LIMIT>  Max output lines [default: 10]
  -v, --verbose...                   Increase logging verbosity
  -q, --quiet...                     Decrease logging verbosity
  -h, --help                         Print help
  -V, --version                      Print version
```