import argparse
from sqlalchemy import create_engine, Table, Column, Integer, String, MetaData, Boolean, ForeignKey, DateTime
from sqlalchemy.dialects.postgresql import UUID
from sqlalchemy.sql import select
from google.cloud import storage
from sqlalchemy import and_, not_
import os

metadata = MetaData()

vods = Table('vods', metadata,
    Column('match_uuid', UUID, nullable=False),
    Column('user_uuid', UUID, nullable=False),
    Column('video_uuid', UUID, nullable=False),
    Column('start_time', DateTime, nullable=False),
    Column('end_time', DateTime, nullable=False),
    Column('raw_container_format', String, nullable=False),
    Column('is_clip', Boolean, nullable=False),
    Column('is_local', Boolean, nullable=False),
    schema='squadov',
)

vod_metadata = Table('vod_metadata', metadata,
    Column('video_uuid', UUID, ForeignKey('squadov.vods.video_uuid'), nullable=False),
    Column('res_x', Integer, nullable=False),
    Column('res_y', Integer, nullable=False),
    Column('min_bitrate', Integer, nullable=False),
    Column('avg_bitrate', Integer, nullable=False),
    Column('max_bitrate', Integer, nullable=False),
    Column('fps', Integer, nullable=False),
    Column('id', String, nullable=False),
    Column('has_fastify', Boolean, nullable=False),
    Column('has_preview', Boolean, nullable=False),
    schema='squadov',
)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--jdbc', required=True)
    parser.add_argument('--bucket', required=True)
    parser.add_argument('--project', required=True)
    args = parser.parse_args()

    engine = create_engine(args.jdbc)
    metadata.create_all(engine)

    s = select(vod_metadata.c.video_uuid, vods.c.raw_container_format).select_from(
        vod_metadata.join(vods, vod_metadata.c.video_uuid == vods.c.video_uuid)
    ).where(
        and_(
            not_(vods.c.is_local),
            vod_metadata.c.has_fastify
        )
    )

    conn = engine.connect()
    result = conn.execute(s)

    gcsClient = storage.Client(project=args.project)
    bucket = gcsClient.get_bucket(args.bucket)

    for row in result:
        print('Delete: ', row.video_uuid)
        if row.raw_container_format == "mp4":
            extension = "mp4"
        else:
            extension = "ts"

        blob = bucket.blob('{}/source/video.{}'.format(row.video_uuid, extension))
        try:
            blob.delete()
        except Exception as e:
            print('Failed to delete: ', e)