import argparse
import csv
import requests

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--csv', required=True)
    parser.add_argument('--token', required=True)
    args = parser.parse_args()

    with open(args.csv) as classes:
        reader = csv.DictReader(classes)
        for row in reader:
            if 'id' not in row:
                continue
            url = "https://api.getvero.com/api/v2/users/delete"
            data = {
                'auth_token': args.token,
                'id': row['id']
            }
            resp = requests.post(url, data=data)