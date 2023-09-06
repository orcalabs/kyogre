import os
import sys
import glob
import json
import math
import time
import xarray
import asyncio
import aiohttp
import aiofiles
import threddsclient
from multiprocessing import Pool


OPERATIONAL_ARCHIVE_URL = "https://thredds.met.no/thredds/catalog/metpparchive/catalog.xml"
HISTORICAL_ARCHIVE_URL = "https://thredds.met.no/thredds/catalog/metpparchivev1/catalog.xml"


def get_met_netcdf_file_urls(archive_url: str, cache_file: str | None = None):
    if cache_file and os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            return json.load(f)

    data_catalog = threddsclient.read_url(archive_url)

    urls = [
        file.download_url()
        for year in data_catalog.flat_references()
        for month in year.follow().flat_references()
        for day in month.follow().flat_references()
        for file in day.follow().flat_datasets()
        if "forecast" not in file.name and "latest" not in file.name
    ]

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
                print(f"Error downloading url '{url}', status: {res.status}, error: {text}")
                return

            async with aiofiles.open(filename, "wb") as f:
                async for chunk in res.content.iter_chunked(1_000_000):
                    await f.write(chunk)


def downscale_and_convert_to_csv(read_file: str, write_file: str):
    try:
        with xarray.open_dataset(read_file) as ds:
            df = ds.to_dataframe()

        resolution = 0.1
        def to_bin(x): return math.floor(x / resolution) * resolution

        df["latitude_bin"] = df.latitude.map(to_bin)
        df["longitude_bin"] = df.longitude.map(to_bin)

        binned_data = df.groupby(["latitude_bin", "longitude_bin"]).mean()
        binned_data.to_csv(write_file, index=False)
    except Exception:
        print(f"Convert Failed: {read_file}")
    finally:
        os.remove(read_file)


def url_to_filename(url: str) -> str:
    # Use the date part at the end as the filename
    return 'data/' + url.split("_")[-1]


def download_only(url: str):
    filename = url_to_filename(url)
    filename_temp = filename + '.tmp'

    start = time.time()
    asyncio.run(download_file(url, filename_temp))
    os.rename(filename_temp, filename)
    end = time.time()

    print(f"Download {filename} in {(end - start):0.2f} seconds")


def _convert_only(file: str):
    start = time.time()
    downscale_and_convert_to_csv(file, file + '.csv')
    end = time.time()
    print(f"Convert {file} in {(end - start):0.2f} seconds")


async def convert_only():
    while True:
        files = glob.glob("data/*.nc")
        if len(files) == 0:
            print("Sleeping...")
            await asyncio.sleep(60)
            continue

        print(f"To Convert: {len(files)}")
        with Pool(min(2, len(files))) as p:
            p.map(_convert_only, files)


def download_and_convert(url: str):
    filename = url_to_filename(url)
    filename_temp = filename + '.tmp'
    filename_csv = filename + '.csv'

    start = time.time()

    d_start = time.time()
    asyncio.run(download_file(url, filename_temp))
    os.rename(filename_temp, filename)
    d_end = time.time()
    download_time = d_end - d_start

    c_start = time.time()
    downscale_and_convert_to_csv(filename, filename_csv)
    c_end = time.time()
    csv_time = c_end - c_start

    end = time.time()
    total = end - start
    print(f"Time:     {total:0.2f} seconds")
    print(f"Download: {((download_time / total) * 100):0.2f} %")
    print(f"Convert:  {((csv_time / total) * 100):0.2f} %")


if __name__ == '__main__':
    # Example url: https://thredds.met.no/thredds/fileServer/metpparchivev1/2019/05/01/met_analysis_1_0km_nordic_20190501T00Z.nc

    # urls = get_met_netcdf_file_urls(OPERATIONAL_ARCHIVE_URL, "operational_urls.json")
    # bad_urls = get_met_netcdf_file_urls(OPERATIONAL_ARCHIVE_URL, "bad_operational_urls.json")
    urls = get_met_netcdf_file_urls(HISTORICAL_ARCHIVE_URL, "historical_urls.json")

    # new_bad = [url for url in urls if url.split('_')[-1] in [
    #     ]]
    # print(new_bad)
    # exit(0)

    # for url in bad_urls:
    #     csv = 'data/' + url.split('_')[-1] + '.csv'
    #     zip = csv + '.zip'
    #     if os.path.isfile(csv):
    #         os.remove(csv)
    #     if os.path.isfile(zip):
    #         os.remove(zip)
    # exit(0)

    # urls = [i for i in urls if "2022/" in i]
    # urls = [i for i in urls if "01/met" in x]
    urls = [
        i
        for i in urls
        if not os.path.isfile('data/' + i.split('_')[-1] + '.nc')
        and not os.path.isfile('data/' + i.split('_')[-1] + '.csv')
        and not os.path.isfile('data/' + i.split('_')[-1] + '.csv.zip')
        # and "2018" not in i
        # and i not in bad_urls
    ]

    print(len(urls))

    if len(sys.argv) == 1:
        with Pool(64) as p:
            p.map(download_and_convert, urls)
    elif sys.argv[1] == 'd':
        with Pool(64) as p:
            p.map(download_only, urls)
    elif sys.argv[1] == 'c':
        asyncio.run(convert_only())
