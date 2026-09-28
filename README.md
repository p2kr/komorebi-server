# komorebi-server

Komorebi is an anime media server and automated downloader. It integrates with AniList and MyAnimeList (MAL) for
metadata, includes built-in crawlers, supports both torrent and direct downloading, and parses anime filenames using
Anitomy. It also features a web dashboard and media streaming capabilities.

## Features

- **Metadata Integration**: Automatically fetches anime metadata from [AniList](https://anilist.co)
  and [MyAnimeList (MAL)](https://myanimelist.net).
- **Automated Crawling**: Built-in crawlers to find the latest episodes and releases.
- **Flexible Downloading**: Supports downloading via Torrents and Direct Download Links (DDL).
- **Smart Parsing**: Uses Anitomy to accurately parse anime filenames.
- **Media Streaming**: Stream your downloaded media directly from the built-in server.
- **Web Dashboard**: An intuitive interface to manage your library, downloads, and settings.

## Built With

- [Go](https://go.dev/) - The programming language
- [Echo v5](https://github.com/labstack/echo) - Web framework
- [Rain](https://github.com/cenkalti/rain) - BitTorrent client and library
- [GoQuery](https://github.com/PuerkitoBio/goquery) - HTML parsing and scraping
- [Anitogo](https://github.com/nssteinbrenner/anitogo) - Go port of Anitomy for anime filename parsing
- [FFmpeg-go](https://github.com/u2takey/ffmpeg-go) - Go bindings for FFmpeg
- [Grab](https://github.com/cavaliergopher/grab) - Download manager package for Go

## Setup & Installation

### Prerequisites

- [Go](https://go.dev/doc/install) installed on your system.

### Installation Steps

1. Clone the repository and navigate to the project directory.
2. Download and install dependencies:
   ```bash
   go mod tidy
   ```
3. Environment Configuration:
   Create a `.env` file in the root directory and configure any necessary API keys (like your MAL/AniList credentials)
   or database connection strings.

## Running the Server

Start the server locally (defaults to port `8080`):

```bash
go run main.go
```

The server includes graceful shutdown handling. Simply press `Ctrl+C` in your terminal to safely stop the application.

## Project Structure

- `src/adapters/` - External API clients for metadata (AniList, MAL)
- `src/controllers/` - HTTP route handlers (Dashboard, Stream, Vault)
- `src/crawlers/` - Web scraping and search engines
- `src/downloaders/` - Logic for managing Torrent and Direct downloads
- `src/parsers/` - Filename parsing using Anitomy
- `src/workers/` - Background tasks, download tracking, and post-download processing
