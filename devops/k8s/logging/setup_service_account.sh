PROFILE=$1
AWS_ACCOUNT_ID=$(aws sts get-caller-identity --profile $PROFILE | jq -r '.Account')
CLUSTER_NAME="primary-eks-cluster"

eksctl delete iamserviceaccount \
--name cloudwatch-agent \
--namespace amazon-cloudwatch \
--cluster $CLUSTER_NAME \
--profile $PROFILE \
--wait

eksctl create iamserviceaccount \
--name cloudwatch-agent \
--namespace amazon-cloudwatch \
--cluster $CLUSTER_NAME \
--attach-policy-arn arn:aws:iam::aws:policy/CloudWatchAgentServerPolicy \
--attach-policy-arn arn:aws:iam::$AWS_ACCOUNT_ID:policy/eks-policy-cloudwatch-logs \
--override-existing-serviceaccounts \
--profile $PROFILE \
--approve