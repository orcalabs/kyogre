import os
import sys
import math
import xarray
import asyncio
import aiohttp
import aiofiles
import threddsclient
from datetime import datetime, timezone


OPERATIONAL_ARCHIVE_URL = "https://thredds.met.no/thredds/catalog/metpparchive/catalog.xml"


def get_met_netcdf_file_urls(latest: datetime) -> list[str]:
    data_catalog = threddsclient.read_url(OPERATIONAL_ARCHIVE_URL)

    urls = []
    for year in data_catalog.flat_references():
        for month in year.follow().flat_references():
            for day in month.follow().flat_references():
                for file in day.follow().flat_datasets():
                    if "forecast" in file.name or "latest" in file.name:
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


def downscale_and_convert_to_csv(read_file: str, write_file: str):
    with xarray.open_dataset(read_file) as ds:
        df = ds.to_dataframe()

    resolution = 0.1
    def to_bin(x): return math.floor(x / resolution) * resolution

    df["latitude_bin"] = df.latitude.map(to_bin)
    df["longitude_bin"] = df.longitude.map(to_bin)

    binned_data = df.groupby(["latitude_bin", "longitude_bin"]).mean()
    binned_data.to_csv(write_file, index=False)


async def download_and_convert(url: str) -> str:
    filename = 'weather_data/' + url.split("_")[-1]
    filename_csv = filename + '.csv'

    await download_file(url, filename)
    downscale_and_convert_to_csv(filename, filename_csv)
    os.remove(filename)

    return filename_csv


def datetime_from_url(url: str) -> datetime:
    date_part = url.split('_')[-1].split('.')[0]
    dt = datetime.strptime(date_part, "%Y%m%dT%HZ")
    dt = dt.replace(tzinfo=timezone.utc)
    return dt


def main(latest_datetime: datetime) -> list[str]:
    if not os.path.isdir('weather_data'):
        os.mkdir("weather_data")

    urls = get_met_netcdf_file_urls(latest_datetime)

    filenames = []
    for url in urls:
        filename = asyncio.run(download_and_convert(url))
        filenames.append(filename)

    return filenames


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: py main.py <latest_timestamp>")
        exit(1)

    latest_datetime = datetime.strptime(sys.argv[1], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    main(latest_datetime)
