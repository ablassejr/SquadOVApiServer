kubectl delete hpa wow-kafka-workers
kubectl autoscale deployment wow-kafka-workers --cpu-percent=50 --min=1 --max=1