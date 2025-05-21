make gen:
	cargo build
	rm -f ~/.local/bin/protoc-gen-pydantic
	mv -fv target/debug/protoc-gen-pydantic ~/.local/bin/protoc-gen-pydantic
	protoc -I protos example.proto --pydantic_out=test
