# kaikichat.com: the static pages, served by nginx behind Coolify's proxy.
# og.png is rendered from og.html (see its header); og.html is not served.
# i18n/ holds a table per language of the app; site.js picks one.
FROM nginx:alpine
COPY deploy/site/nginx.conf /etc/nginx/conf.d/default.conf
COPY deploy/site/index.html deploy/site/privacy.html deploy/site/style.css deploy/site/favicon.svg deploy/site/og.png deploy/site/network.json deploy/site/install.sh deploy/site/site.js /usr/share/nginx/html/
COPY deploy/site/i18n/ /usr/share/nginx/html/i18n/
