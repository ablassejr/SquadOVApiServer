#!/bin/bash
set -xe

# Obtain Let's Encrypt certificate at first so we can get the proper HTTPS configuration.
CERT_FILE=/etc/letsencrypt/live/staging.squadov.gg/cert.pem
if [[ -f "$CERT_FILE" ]]; then
    certbot renew -n --standalone
else
    certbot certonly -n --standalone -d ${DEPLOYMENT_DOMAIN} -d api.${DEPLOYMENT_DOMAIN} -d app.${DEPLOYMENT_DOMAIN} -d auth.${DEPLOYMENT_DOMAIN} --agree-tos --email ${DEPLOYMENT_DOMAIN_EMAIL} --redirect
fi

service cron start
nginx -g 'daemon off;'