import os
import sys
import math
import xarray
import asyncio
import aiohttp
import aiofiles
import threddsclient
import pandas as pd
from datetime import datetime, timezone


ARCHIVE_URL = "https://thredds.met.no/thredds/catalog/fou-hi/norkyst800m-1h/catalog.xml"
DATA_DIR = 'ocean_data/'
DEPTHS = [0, 25, 500]


def datetime_from_url(url: str) -> datetime:
    date_part = url.split('.')[-2]
    dt = datetime.strptime(date_part, "%Y%m%d%H")
    dt = dt.replace(tzinfo=timezone.utc)
    return dt


def get_met_netcdf_file_urls(latest: datetime) -> list[str]:
    data_catalog = threddsclient.read_url(ARCHIVE_URL)

    urls = []
    for file in data_catalog.flat_datasets():
        if ".fc." in file.name:
            continue

        url = file.download_url()
        if datetime_from_url(url) <= latest:
            return urls

        urls.append(url)

    return urls


async def download_file(url: str, filename='temp'):
    timeout = aiohttp.ClientTimeout(total=100_000)
    async with aiohttp.ClientSession(timeout=timeout) as session:
        async with session.get(url) as res:
            if res.status != 200:
                text = await res.text()
                print(f"Error downloading url '{url}', status: {res.status}, error: {text}")
                return

            async with aiofiles.open(filename, "wb") as f:
                async for chunk in res.content.iter_chunked(1_000_000):
                    await f.write(chunk)


def to_bin(x: float) -> int:
    return int(math.floor(x * 10))


def downscale_and_convert_to_csv(ds: xarray.Dataset, csv_file: str):
    df = ds.to_dataframe()

    df["lat_bin"] = df.lat.map(to_bin)
    df["lon_bin"] = df.lon.map(to_bin)

    data = df.groupby(['lat_bin', 'lon_bin']).mean()
    data.to_csv(csv_file, index=False)


def split_and_downscale_and_convert_to_csv(file: str) -> list[str]:
    csv_files = []

    with xarray.open_dataset(file) as ds:
        ds = ds.drop_dims(['s_rho', 's_w'])
        ds = ds.reset_coords(['lat', 'lon'])

        for (depth, group) in ds.groupby('depth'):
            if int(depth) not in DEPTHS:
                continue

            for (time, group) in group.groupby('time'):
                csv_file = f"{DATA_DIR}{pd.to_datetime(time).strftime('%Y%m%d%H')}_{int(depth)}.nc.csv"
                downscale_and_convert_to_csv(group, csv_file)
                csv_files.append(csv_file)

    os.remove(file)
    return csv_files


async def download_and_convert(url: str) -> list[str]:
    file = DATA_DIR + url.split(".")[-2] + '.nc'

    await download_file(url, file)
    csv_files = split_and_downscale_and_convert_to_csv(file)

    return csv_files


def main(latest_datetime: datetime) -> list[str]:
    if not os.path.isdir(DATA_DIR):
        os.mkdir(DATA_DIR)

    urls = get_met_netcdf_file_urls(latest_datetime)

    filenames = []
    for url in urls:
        files = asyncio.run(download_and_convert(url))
        filenames.extend(files)

    return filenames


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: py main.py <latest_timestamp>")
        exit(1)

    latest_datetime = datetime.strptime(sys.argv[1], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    filenames = main(latest_datetime)
    print(filenames)
