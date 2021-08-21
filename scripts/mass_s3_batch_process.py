import argparse
import csv

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--csv', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()

    data = []
    with open(args.csv) as iff:
        reader = csv.DictReader(iff)

        with open(args.output, 'w', newline='') as off:
            writer = csv.writer(off)

            for row in reader:
                bucket = row['bucket'].replace('s3://', '')

                writer.writerow([
                    bucket,
                    '{}/source/video.mp4'.format(row['video_uuid'])
                ])

                writer.writerow([
                    bucket,
                    '{}/source/video.ts'.format(row['video_uuid'])
                ])

                writer.writerow([
                    bucket,
                    '{}/source/fastify.mp4'.format(row['video_uuid'])
                ])

                writer.writerow([
                    bucket,
                    '{}/source/preview.mp4'.format(row['video_uuid'])
                ])

                writer.writerow([
                    bucket,
                    '{}/source/thumbnail.jpg'.format(row['video_uuid'])
                ])