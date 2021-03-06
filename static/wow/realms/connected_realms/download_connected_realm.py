import argparse
import json
import os
import requests

parser = argparse.ArgumentParser()
parser.add_argument('--region', required=True)
parser.add_argument('--token', required=True)
parser.add_argument('--output', required=True)
args = parser.parse_args()

region = args.region
namespace = 'dynamic-{}'.format(region)
token = args.token

if not os.path.exists(args.output):
    os.makedirs(args.output)

connectedIndexUrl = 'https://{}.api.blizzard.com/data/wow/connected-realm/index?namespace={}&locale=en_US&access_token={}'.format(region, namespace, token)
r = requests.get(connectedIndexUrl).json()
for c in r["connected_realms"]:
    url = c["href"]
    url = '{}&access_token={}'.format(url, token)

    rr = requests.get(url).json()
    with open(os.path.join(args.output, '{}.json'.format(rr["id"])), 'w') as f:
        json.dump(rr, f)