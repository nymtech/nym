FROM node
RUN apt update && apt install -y netcat
COPY entrypoint.sh /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
