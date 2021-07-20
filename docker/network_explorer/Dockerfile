FROM node:14
ARG PERSONAL_TOKEN
RUN git clone "https://oauth2:${PERSONAL_TOKEN}@gitlab.nymte.ch/jessgess/network-explorer.git"

WORKDIR network-explorer
COPY ./setup.sh /setup.sh
RUN /setup.sh
CMD yarn dev
