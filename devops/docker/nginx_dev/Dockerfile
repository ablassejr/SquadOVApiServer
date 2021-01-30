FROM nginx:1.19.3
RUN yes | rm /etc/nginx/conf.d/*
RUN apt-get update && apt-get install -y curl
RUN curl https://ssl-config.mozilla.org/ffdhe2048.txt > /etc/nginx/conf.d/dhparam.pem
COPY run.sh .
COPY nginx.conf /etc/nginx/conf.d/nginx.conf
COPY common-http-headers.conf /etc/nginx/conf.d/common-http-headers.conf
EXPOSE 80/tcp
ENTRYPOINT ["bash", "run.sh"]