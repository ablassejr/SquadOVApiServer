from google.oauth2 import service_account
import googleapiclient.discovery
import csv
import argparse
import datetime
import time

#define the scope
SCOPES = ['https://www.googleapis.com/auth/analytics.user.deletion']

# I am using a service account to call the API 
SERVICE_ACCOUNT_FILE = '../devops/gcp/squadov_ga.json'# Path to your service account credentials

#Load up the credentials
credentials = service_account.Credentials.from_service_account_file(
  SERVICE_ACCOUNT_FILE,
  scopes=SCOPES
)

#build the service object
analytics_client = googleapiclient.discovery.build(
  'analytics',
  'v3',
  credentials=credentials
)

#initialise
user_deletion_request_resource = analytics_client.userDeletion().userDeletionRequest()

# this is where the action happens:
def delete_users(id):
    return user_deletion_request_resource.upsert(
        body = {
            "deletionRequestTime": str(datetime.datetime.now()),# This marks the point in time at which Google received the deletion request
            "kind": "analytics#userDeletionRequest",  # Value is "analytics#userDeletionRequest"
            "id": {  # User ID Object.
                "userId": id,  # The User's id
                "type": "CLIENT_ID",  # Type of user (APP_INSTANCE_ID,CLIENT_ID or USER_ID)
            },
            "webPropertyId": "UA-185942570-1"  # Web property ID of the form UA-XXXXX-YY.
        }
    ).execute()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--csv', required=True)
    args = parser.parse_args()

    with open(args.csv) as classes:
        reader = csv.DictReader(classes)
        for row in reader:
            res = delete_users(row['Client Id'])
            time.sleep(0.05)