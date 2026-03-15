FROM rust:alpine3.21 AS builder

RUN mkdir /app
WORKDIR /app

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig curl pipx openjdk11

COPY ./seerr-api.yml .

RUN pipx install openapi-generator-cli==7.20 && \
    pipx run openapi-generator-cli==7.20 generate -i "https://raw.githubusercontent.com/Radarr/Radarr/develop/src/Radarr.Api.V3/openapi.json" -g rust -o $PWD/openapi_generated/radarr --additional-properties=packageName=radarr,library=reqwest-trait,supportAsync=true,useSingleRequestParameter=true,topLevelApiClient=true,useBonBuilder=true,enumNameSuffix=Radarr --model-name-prefix=Radarr --global-property=apis=ApiInfo:Movie,models,supportingFiles,apiDocs=false,modelDocs=false --remove-operation-id-prefix && \
    pipx run openapi-generator-cli==7.20 generate -i "https://raw.githubusercontent.com/Sonarr/Sonarr/develop/src/Sonarr.Api.V3/openapi.json" -g rust -o $PWD/openapi_generated/sonarr --additional-properties=packageName=sonarr,library=reqwest-trait,supportAsync=true,useSingleRequestParameter=true,topLevelApiClient=true,useBonBuilder=true,enumNameSuffix=Sonarr --model-name-prefix=Sonarr --global-property=apis=ApiInfo:Series,models,supportingFiles,apiDocs=false,modelDocs=false --remove-operation-id-prefix && \
    pipx run openapi-generator-cli==7.20 generate -i "$PWD/seerr-api.yml" -g rust -o $PWD/openapi_generated/seerr --additional-properties=packageName=seerr,library=reqwest-trait,supportAsync=true,useSingleRequestParameter=true,topLevelApiClient=true,useBonBuilder=true,enumNameSuffix=Seerr --model-name-prefix=Seerr --global-property=apis=Request:Settings:Users:Movies:Tv,models,supportingFiles,apiDocs=false,modelDocs=false --remove-operation-id-prefix

COPY ./Cargo.toml ./Cargo.lock .

RUN mkdir -pv src && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build -r && \
    rm -Rvf src

COPY ./src ./src/

RUN touch src/main.rs && cargo build -r

FROM alpine:3.21 AS runner

COPY --from=builder /app/target/release/informarr /informarr

RUN apk add --no-cache tini libgcc

WORKDIR /config

EXPOSE 3000

ENTRYPOINT ["tini", "--"]
CMD ["/informarr"]
