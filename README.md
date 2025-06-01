## unnest-ndjson

[![Crates.io Version](https://img.shields.io/crates/v/unnest-ndjson)](https://crates.io/crates/unnest-ndjson)

This tool can unpack JSON objects into `ndjson`, also called [jsonlines](https://jsonlines.org/).

`ndjson` is much easier to consume than JSON objects in some situations.


### Usage

* `TARGET_DEPTH`: how many levels of document to strip away
* `--path`: include the path to the element, as the `key`


### Examples

Say you have a JSON document that looks like:

```json
[
  {"name": "john", "class": "warrior"},
  {"name": "sam", "class": "wizard"},
  {"name": "alex", "class": "terrible"}
]
```

You could produce:
```
<array.json unnest-ndjson 1
```

```json lines
{"name":"john","class":"warrior"}
{"name":"sam","class":"wizard"}
{"name":"alex","class":"terrible"}
```

That is, removing the outer array wrapper.

Or, with `--path`, it can produce:
```json lines
{"key":[0],"value":{"name":"john","class":"warrior"}}
{"key":[1],"value":{"name":"sam","class":"wizard"}}
{"key":[2],"value":{"name":"alex","class":"terrible"}}
```

Or, with `--path 2`, it can produce:
```json lines
{"key":[0,"name"],"value":"john"}
{"key":[0,"class"],"value":"warrior"}
{"key":[1,"name"],"value":"sam"}
{"key":[1,"class"],"value":"wizard"}
{"key":[2,"name"],"value":"alex"}
{"key":[2,"class"],"value":"terrible"}

```

A similar thing works for non-array documents, like:
```json
{
  "john": {"class": "warrior"},
  "sam": {"class": "wizard"},
  "alex": {"class": "terrible"}
}
```

You might want:
```json lines
{"key":["john"],"value":{"class":"warrior"}}
{"key":["sam"],"value":{"class":"wizard"}}
{"key":["alex"],"value":{"class":"terrible"}}
```


### Why?

It's quite fast, and uses very little memory. This could be useful if you wanted
to process the resulting, much smaller, JSON documents with another application.

On 2019 hardware, it can process JSON at about 1GB/s, and needs approximately no memory to do so.

A 300MB file can be converted in 300ms and 3MB of RAM, regardless of settings.
For comparison, `jq` takes over 2 *seconds*, and 350MB of RAM, to *read* the same file,
even if it is not printing any of it.


### How?

It's a custom JSON "parser" (scanner? bracket matcher?), which doesn't try and
actually load the JSON into memory, or decode any of the idiosyncrasies.


### License

`MIT OR Apache-2.0`
