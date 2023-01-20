# Docker
Build: `docker build -t nes-cloud -f nes-cloud/Dockerfile .`

Run: `docker run -p 4444:4444 -p 5555:5555 -p 6666:6666 -p 7777:7777 --init -it --rm nes-cloud`

Shell: `docker run --init -it --rm nes-cloud bash`

Running: `docker exec -it [ID] /bin/bash`

Compose: `docker compose -f nes-cloud/docker-compose.yaml up`


# Deploy
1. `cd potatis`
2. `docker build --platform linux/amd64 -t http://registry.fly.io/nes-cloud:TAG -f nes-cloud/Dockerfile .`
3. `docker push http://registry.fly.io/nes-cloud:TAG`
4. `flyctl deploy -i http://registry.fly.io/nes-cloud:TAG -a nes-cloud`