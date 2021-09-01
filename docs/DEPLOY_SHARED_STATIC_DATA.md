# Deploy to GCP

This guide assumes you have deployment access to the `squadov-static` on GCP.

## Generate Static Data

### Hearthstone

Ensure that you have the `AssetStudio` project checked out.
We will refer to the folder as `$ASSET`.

1. Run the modified `AssetStudio` program.
2. Select `File > Load Folder` and open the Hearthstone data folder (i.e. `C:\Program Files (x86)\Hearthstone\Data\Win`)
3. Select `Export > Export Hearthstone Assets` to a folder. This folder will be referred to as `$HSD`.

In the export folder you should see the following file structure:
- Cards
  - CARD_ID
    - metadata.json
    - portrait.png
- CardBacks
  - CARD_BACK_ID
    - metadata.json
    - texture.png

We will now need to pull pre-rendered cards from Blizzard's API.
Ensure that you have the `SquadOvApiServer` project checked out (hereby referred to as `$SRC`).

1. `cd $SRC/scripts`
2. `python mass_pull_hearthstone_data.py --folder $HSD --clientId BLIZZARD_OAUTH_CLIENT_ID --clientSecret BLIZZARD_OAUTH_CLIENT_SECRET`

This script will take awhile to run.
After it finishes running, in each `CARD_ID` folder, you should see an additional `card.png` and possibly a `cardGold.png`.
In each `CARD_BACK_ID` folder, you should see an additional `back.png`.

And to upload

1. Create a new bucket named `us-central1.content.squadov.gg` where `$ENV` should be the GCP project name and `us-central1` should be named appropriately for the location of the bucket.
2. Create a GCS folder named `hearthstone`.
3. `gsutil rsync -x ".*\.json$" -r $HSD gs://us-central1.content.squadov.gg/hearthstone`

### TFT (Champions, Items, Traits)

Download the latest TFT static data set from Riot Game's developer portal.

1. `python .\parse_tft_champions_data.py --json A:\Git\TftAssets\set5patch1115\champions.json --assets A:\Git\TftAssets\set5patch1115\champions --output A:\Git\TftAssets\Organized_Sets\set5.5\champions`
2. `python .\parse_tft_item_data.py --json A:\Git\TftAssets\set5patch1115\items.json --assets A:\Git\TftAssets\set5patch1115\items --output A:\Git\TftAssets\Organized_Sets\set5.5\items`
3. `python .\parse_tft_traits_data.py --json A:\Git\TftAssets\set5patch1115\traits.json --assets A:\Git\TftAssets\set5patch1115\traits --output A:\Git\TftAssets\Organized_Sets\set5.5\traits`

And to upload

`gsutil -m rsync -r ./set5.5/ gs://us-central1.content.squadov.gg/tft/set5.5`

### TFT (Little Legends)

1. Install `https://github.com/Crauzer/Obsidian`
2. In Obisidian, open `C:\Riot Games\League of Legends\Plugins\rcp-be-lol-game-data\default-assets.wad` 
3. Extract `plugins/rcp-be-lol-game-data/global/default/v1/companions.json` and `plugins/rcp-be-lol-game-data/global/default/assets/{loadouts,loot}/companions/*`
4. `python .\parse_little_legends_data.py --json "A:\Git\TftAssets\Extracted\plugins\rcp-be-lol-game-data\global\default\v1\companions.json" --assets "A:\Git\TftAssets\Extracted\plugins\rcp-be-lol-game-data\global\default\assets" --output "A:\Git\TftAssets\Organized_Companions"`

And to upload:

`gsutil -m rsync -r Organized_Companions gs://us-central1.content.squadov.gg/tft/companions`

## Setup GCS Load Balancer

1. Create a new HTTP(S) Load Balancer by going to `Network services > Load balancing` using the following options:
  - From Internet to my VMs
  - Name: `squadov-static-lb`
  - Backend Configuration: `Backend Buckets > Create a new bucket`.
    - Name: `squadov-static-lb-bucket`
    - Cloud Storage Bucket: `us-central1.content.squadov.gg`
  - Frontend Configuration:
    - Name: `squadov-static-lb-frontend`
    - Protocol: `HTTPS`
    - Network Service Tier: `Premium`
    - IP Version: `IPv4`
    - IP address: `Create IP Address`
      - Name: `squadov-static-lb-frontend-ip`
    - Port: 443
    - Certificate: `Create a new certificate`
      - Name: `squadov-static-lb-frontend-cert`
      - Create Google-managed certificate
      - Domain: `us-central1.content.squadov.gg`
2. Add an `A` record on Cloudflare to the load balancer's IP for the `us-central1.content` subdomain.

## Upload Metadata to Database

1. `cd $SRC/scripts`
2. `python sync_hearthstone_metadata.py --folder $HSD --jdbc $JDBC` where `$JDBC` is a JDBC URL to connect to the database (whether it's a local one or one you connect to via Cloud SQL Proxy).