set PROTOC_INCLUDE=D:\Hider\Code\LocalImageSearch\rust-extension\protoc-include
set PROTOC=D:\Hider\Code\LocalImageSearch\rust-extension\protoc.exe
cargo build
copy ".\target\debug\ai-search-extension.exe" D:\Hider\Code\LocalImageSearch\extensions\