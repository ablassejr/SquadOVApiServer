import argparse
from sqlalchemy import create_engine, Table, Column, Integer, String, MetaData, Boolean, ForeignKey, DateTime
from sqlalchemy.sql import select
from sqlalchemy import and_
import subprocess
import json

metadata = MetaData()

riot_accounts = Table('riot_accounts', metadata,
    Column('puuid', String, nullable=False),
    Column('game_name', String, nullable=True),
    Column('tag_line', String, nullable=True),
    Column('account_id', DateTime, nullable=True),
    Column('summoner_id', DateTime, nullable=True),
    Column('summoner_name', String, nullable=True),
    Column('last_backfill_tft_time', DateTime, nullable=False),
    Column('last_backfill_lol_time', DateTime, nullable=False),
    Column('raw_puuid', String, nullable=True),
    schema='squadov',
)

riot_account_links = Table('riot_account_links', metadata,
    Column('puuid', String, ForeignKey('squadov.riot_accounts.puuid'), nullable=False),
    Column('user_id', Integer, nullable=False),
    Column('rso_access_token', String, nullable=True),
    Column('rso_refresh_token', String, nullable=True),
    Column('rso_expiration', DateTime, nullable=True),
    schema='squadov',
)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--jdbc', required=True)
    parser.add_argument('--username', required=True)
    parser.add_argument('--password', required=True)
    args = parser.parse_args()

    engine = create_engine(args.jdbc)
    metadata.create_all(engine)

    s = select(riot_account_links.c.rso_access_token, riot_account_links.c.rso_refresh_token, riot_account_links.c.rso_expiration, riot_account_links.c.user_id).select_from(
        riot_account_links.join(riot_accounts, riot_accounts.c.puuid == riot_account_links.c.puuid)
    ).where(
        and_(
            riot_accounts.c.summoner_name == None,
            riot_account_links.c.rso_access_token != None,
            riot_account_links.c.rso_refresh_token != None,
            riot_account_links.c.rso_expiration != None,
        )
    )

    conn = engine.connect()
    result = conn.execute(s)

    for row in result:
        packet = json.dumps({
            'type': 'AccountMe',
            'access_token': row.rso_access_token,
            'refresh_token': row.rso_refresh_token,
            'expiration': row.rso_expiration.isoformat(),
            'user_id': row.user_id,
        })
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
            'routing_key=riot_rso',
            'payload={}'.format(packet)
        ]

        print(cmd)
        subprocess.call(cmd)