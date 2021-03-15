import argparse
import json
import subprocess

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--json', required=True)
    parser.add_argument('--username', required=True)
    parser.add_argument('--password', required=True)
    args = parser.parse_args()

    with open(args.json, 'r') as f:
        data = json.load(f)
    
    for d in data:
        cmd = [
            'rabbitmqadmin',
            '--host=albatross.rmq.cloudamqp.com',
            '--port=443',
            '--username={}'.format(args.username),
            '--password={}'.format(args.password),
            '--ssl',
            '--vhost={}'.format(args.username),
            'publish',
            'exchange=amq.default',
            'routing_key=squadov_vods',
            'payload={}'.format(
                json.dumps({
                    'type': 'Process',
                    'vod_uuid': d,
                })
            )
        ]

        print(cmd)
        subprocess.call(cmd)