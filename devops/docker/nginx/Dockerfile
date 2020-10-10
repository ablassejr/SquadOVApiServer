FROM nginx:1.19.3
RUN yes | rm /etc/nginx/conf.d/*
RUN apt-get update && apt-get install -y certbot python3-certbot-nginx
RUN apt-get install -y cron curl
RUN curl https://ssl-config.mozilla.org/ffdhe2048.txt > /etc/nginx/conf.d/dhparam.pem
COPY run.sh .
COPY nginx.conf /etc/nginx/conf.d/nginx.conf
COPY common-http-headers.conf /etc/nginx/conf.d/common-http-headers.conf
COPY common-tls-options.conf /etc/nginx/conf.d/common-tls-options.conf
EXPOSE 80/tcp
EXPOSE 443/tcp
ENTRYPOINT ["bash", "run.sh"]