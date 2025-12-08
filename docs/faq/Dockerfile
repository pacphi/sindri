# Minimal static file server for Sindri FAQ
# Image size: ~25MB
FROM nginx:alpine

# Remove default nginx content
RUN rm -rf /usr/share/nginx/html/*

# Copy the self-contained FAQ page
COPY index.html /usr/share/nginx/html/

# Nginx runs on port 80 by default
EXPOSE 80
