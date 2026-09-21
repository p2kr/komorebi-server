package core

import (
	"komorebi-server/src/downloaders"
	"komorebi-server/src/workers"
)

func Defer() {
	downloaders.TorrentClient().Close()
	workers.CloseScheduler()
}
