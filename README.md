# remi 
`remi` is a bare-bones [`gemini`](https://geminiprotocol.net/) client.

## Features
* The ability to go back/forward in history
* Bookmarks
* In-app console that displays errors returned from the server

### Work In Progress Features
* Input popups for when the server requests an input query
* Support for all response types (currently only a limited subset of response types defined in the gemini protocol are supported)

## Build Instructions
### Prerequisites
* [Rust & cargo](https://www.rust-lang.org/tools/install)

### Instructions
```console
  $ cargo run --release
```
