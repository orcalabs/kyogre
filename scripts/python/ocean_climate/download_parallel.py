import os
import glob
import time
import sys
import math
import json
import xarray
import asyncio
import aiohttp
import aiofiles
import threddsclient
import pandas as pd
from datetime import datetime, timezone
from multiprocessing import Pool


ARCHIVE_URL = "https://thredds.met.no/thredds/catalog/fou-hi/norkyst800m-1h/catalog.xml"


def datetime_from_url(url: str) -> datetime:
    date_part = url.split('.')[-2]
    dt = datetime.strptime(date_part, "%Y%m%d%H")
    dt = dt.replace(tzinfo=timezone.utc)
    return dt


def get_met_netcdf_file_urls(latest: datetime | None = None, cache_file: str | None = None) -> list[str]:
    if cache_file and os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            return json.load(f)

    data_catalog = threddsclient.read_url(ARCHIVE_URL)

    if not latest:
        urls = [
            file.download_url()
            for file in data_catalog.flat_datasets()
            if ".fc." not in file.name
        ]
        if cache_file:
            with open(cache_file, "w") as f:
                json.dump(urls, f)
        return urls

    urls = []
    for file in data_catalog.flat_datasets():
        if ".fc." in file.name:
            continue

        url = file.download_url()
        if datetime_from_url(url) <= latest:
            return urls

        urls.append(url)

    if cache_file:
        with open(cache_file, "w") as f:
            json.dump(urls, f)

    return urls


async def download_file(url: str, filename='temp'):
    timeout = aiohttp.ClientTimeout(total=100_000)
    async with aiohttp.ClientSession(timeout=timeout) as session:
        async with session.get(url) as res:
            if res.status != 200:
                text = await res.text()
                print(
                    f"Error downloading url '{url}', status: {res.status}, error: {text}")
                return

            async with aiofiles.open(filename, "wb") as f:
                async for chunk in res.content.iter_chunked(10_000_000):
                    await f.write(chunk)


def url_to_filename(url: str) -> str:
    # Use the date part at the end as the filename
    return 'ocean_data/' + url.split(".")[-2] + '.nc'


async def download_only(urls: list[str]):
    for url in urls:
        filename = url_to_filename(url)
        filename_temp = filename + '.tmp'

        start = time.time()
        await download_file(url, filename_temp)
        os.rename(filename_temp, filename)
        end = time.time()

        print(f"Download {filename} in {(end - start):0.2f} seconds")


def to_bin(x):
    return int(math.floor(x * 10))


def downscale_and_convert_to_csv(params: tuple[str, xarray.Dataset]):
    start = time.time()

    csv_file, ds = params
    df = ds.to_dataframe()

    df["lat_bin"] = df.lat.map(to_bin)
    df["lon_bin"] = df.lon.map(to_bin)

    data = df.groupby(['lat_bin', 'lon_bin']).mean()
    data.to_csv(csv_file, index=False)

    end = time.time()
    print(f"Convert {csv_file} in {(end - start):0.2f} seconds")


def _convert_only(file: str):
    start = time.time()
    with xarray.open_dataset(file) as ds:
        ds = ds.drop_dims(['s_rho', 's_w'])
        ds = ds.reset_coords(['lat', 'lon'])

        x = [
            (f"ocean_data/{pd.to_datetime(t).strftime('%Y%m%d%H')}_{int(depth)}.nc.csv", group)
            for (t, group) in ds.groupby('time')
            for (depth, group) in group.groupby('depth')
        ]

    with Pool(16) as p:
        p.map(downscale_and_convert_to_csv, x)

    os.remove(file)
    end = time.time()
    print(f"Process {file} in {(end - start):0.2f} seconds")


async def convert_only():
    while True:
        files = glob.glob(r"ocean_data/*.nc")
        if len(files) == 0:
            print("Sleeping...")
            await asyncio.sleep(60)
            continue

        print(f"To Convert: {len(files)}")
        for f in files:
            _convert_only(f)


if __name__ == "__main__":
    urls = get_met_netcdf_file_urls(cache_file="ocean_urls.json")

    orig_len = len(urls)
    print(f"All urls: {orig_len}")

    existing = set([f[:8] for f in os.listdir('ocean_data')])
    print(f"Existing: {len(existing)}")

    urls = [
        url
        for url in urls
        if url.split('.')[-2][:-2] not in existing
    ]

    if len(urls) != orig_len - len(existing):
        print(f"WARNING! Before: {orig_len}, Existing: {len(existing)}, After: {len(urls)}")
        exit(0)

    print(f"To download: {len(urls)}")

    if len(sys.argv) == 1:
        exit(0)
    elif sys.argv[1] == 'd':
        temp_files = glob.glob('ocean_data/*.tmp')
        for f in temp_files:
            os.remove(f)
        asyncio.run(download_only(urls))
    elif sys.argv[1] == 'c':
        asyncio.run(convert_only())
