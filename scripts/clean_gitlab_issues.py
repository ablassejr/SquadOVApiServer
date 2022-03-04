import argparse
import requests

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--key', required=True)
    args = parser.parse_args()

    while True:
        url = "https://gitlab.com/api/v4/projects/21687296/issues"
        resp = requests.get(url, params={
            'scope': 'all',
            'labels': 'user-reported'
        }, headers={
            'PRIVATE-TOKEN': args.key
        }).json()
        
        if len(resp) == 0:
            break

        for t in resp:
            print('Deleting Ticket: ', t['iid'])
            url = "https://gitlab.com/api/v4/projects/21687296/issues/{}".format(t['iid'])
            dresp = requests.delete(url, headers={
                'PRIVATE-TOKEN': args.key
            })
            print(dresp)