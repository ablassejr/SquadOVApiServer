gcloud container clusters get-credentials --zone us-central1-c squadov-primary-cluster
kubectl create secret generic gke-service-account --from-file=gcloud-service-account.json=../../gcloud/gcloud-kubernetes-account.json -o yaml --dry-run --save-config | kubectl apply -f -
kubectl create secret docker-registry regcred --docker-server=registry.gitlab.com --docker-username=${GITLAB_USERNAME} --docker-password=${GITLAB_REGISTRY_TOKEN} -o yaml --dry-run --save-config | kubectl apply -f -
kubectl create secret generic postgres-secret --from-literal=username=${POSTGRES_USER} --from-literal=password="${POSTGRES_PASSWORD}"
kubectl create secret generic mysql-secret --from-literal=username=${MYSQL_USER} --from-literal=password="${MYSQL_PASSWORD}"
kubectl create secret generic fusionauth-db-secret --from-literal=username=${FUSIONAUTH_DB_USER} --from-literal=password="${FUSIONAUTH_DB_PASSWORD}"
kubectl create secret generic aws-secret --from-literal=AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} --from-literal=AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY}"