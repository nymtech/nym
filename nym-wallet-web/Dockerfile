FROM node
RUN git clone https://github.com/nymtech/nym-wallet-web.git
RUN apt update && apt install -y dnsutils

WORKDIR /nym-wallet-web
RUN yarn install
COPY entrypoint.sh /entrypoint.sh
CMD /entrypoint.sh
