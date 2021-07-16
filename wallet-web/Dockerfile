FROM node
COPY . /wallet-web
RUN apt update && apt install -y dnsutils

WORKDIR /wallet-web
RUN yarn install
CMD ./entrypoint.sh
