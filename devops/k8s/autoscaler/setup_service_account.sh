AWS_ACCOUNT_ID=$(aws sts get-caller-identity | jq -r '.Account')
CLUSTER_NAME="primary-eks-cluster"

eksctl delete iamserviceaccount \
--name cluster-autoscaler \
--namespace kube-system \
--cluster $CLUSTER_NAME \
--wait

eksctl create iamserviceaccount \
--name cluster-autoscaler \
--namespace kube-system \
--cluster $CLUSTER_NAME \
--attach-policy-arn arn:aws:iam::$AWS_ACCOUNT_ID:policy/eks-policy-autoscaler \
--override-existing-serviceaccounts \
--approve