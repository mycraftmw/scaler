.PHONY: binary, proto

default: binary


clean:
	-rm -Rf _output

binary:
	PROTO=1 cargo build


docker-build:
	docker  build --platform=linux/amd64 -f Dockerfile -t scaler .

docker-push:
	export a=$(git show --abbrev-commit --shortstat | head -n1 | awk '{print $NF}')
	echo $a
	# docker tag scaler registry.cn-shanghai.aliyuncs.com/scalar_challenge/scaler:$(git show --abbrev-commit --shortstat | head -n1 | awk '{print $NF}') scaler

