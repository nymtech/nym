FROM node
COPY . /validator
COPY ./upload_contract /upload_contract
RUN rm -rf /validator/upload_contract

WORKDIR /validator
RUN yarn install
RUN yarn build

WORKDIR /upload_contract
RUN npm install

ENTRYPOINT ["./init_and_start.sh"]
