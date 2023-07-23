.PHONY: binary, proto

default: binary


clean:
	-rm -Rf _output

binary:
	PROTO=1 cargo build


docker-build:
#	manifest/make-rules/manifest.sh ${IMAGE_REPO}/edge-proxy:${GIT_VERSION} ${DOCKER_USERNAME} ${DOCKER_PASSWD}
	docker buildx build --push ${DOCKER_BUILD_ARGS} --platform ${TARGET_PLATFORMS} \
    --build-arg TARGETOS=linux --build-arg TARGETARCH=amd64 --build-arg GIT_VERSION=${GIT_VERSION} \
    -f Dockerfile . -t ${IMAGE_REPO}/scaler:${GIT_VERSION}

