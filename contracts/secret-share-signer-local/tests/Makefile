all:
	$(MAKE) -C ../../.. build

build-mpc-bindings-wasm:
	(cd ../../../packages/framework && cargo make bindings-node)

run-localsecret:
	docker run -it --rm -p 1317:1316 --name localsecret ghcr.io/scrtlabs/localsecret:v1.9.3

kill-localsecret:
	docker rm -f localsecret
