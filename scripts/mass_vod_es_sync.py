import argparse
import csv
import subprocess
import json
from multiprocessing import Pool

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--csv', required=True)
    parser.add_argument('--host', required=True)
    parser.add_argument('--queue', required=True)
    parser.add_argument('--username', required=True)
    parser.add_argument('--password', required=True)
    parser.add_argument('--threads', required=True, type=int)
    args = parser.parse_args()

    data = []
    with open(args.csv) as classes:
        reader = csv.DictReader(classes)
        for row in reader:
            if 'video_uuid' not in row or 'game' not in row:
                continue

            if row['game'] == 'NULL':
                continue
            data.append((row['video_uuid'], row['game']))

    def process(data):
        cmd = [
            'rabbitmqadmin',
            '--host={}'.format(args.host),
            '--port=443',
            '--username={}'.format(args.username),
            '--password={}'.format(args.password),
            '--ssl',
            '--vhost={}'.format(args.username),
            'publish',
            'exchange=amq.default',
            'routing_key={}'.format(args.queue),
            'payload={}'.format(
                json.dumps({
                    'type': 'SyncVod',
                    'video_uuid': [data[0]],
                    'game': int(data[1]),
                })
            )
        ]

        print(cmd)
        subprocess.call(cmd)

    with Pool(args.threads) as p:
        p.map(process, data)