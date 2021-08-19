import argparse
import csv
import subprocess
import json

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--csv', required=True)
    parser.add_argument('--queue', required=True)
    parser.add_argument('--username', required=True)
    parser.add_argument('--password', required=True)
    args = parser.parse_args()

    data = []
    with open(args.csv) as classes:
        reader = csv.DictReader(classes)
        for row in reader:
            if 'match_uuid' not in row:
                continue
            data.append(row['match_uuid'])

    for i in range(0, len(data), 100):
        subset = data[i:i+100]
        cmd = [
            'rabbitmqadmin',
            '--host=advanced-green-bird-01.rmq2.cloudamqp.com',
            '--port=443',
            '--username={}'.format(args.username),
            '--password={}'.format(args.password),
            '--ssl',
            '--vhost={}'.format(args.username),
            'publish',
            'exchange=amq.default',
            'routing_key={}'.format(args.queue),
            'payload={}'.format(
                json.dumps(subset)
            )
        ]

        print(cmd)
        subprocess.call(cmd)