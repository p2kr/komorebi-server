package processors

import (
	"context"
	"sync"
	"time"
	"uuid"
)

type Session struct {
	VaultItemId uuid.UUID
	Cancel      context.CancelFunc
	lastRequest time.Time
	mu          sync.Mutex
}

func (s *Session) Touch() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.lastRequest = time.Now()
}

func (s *Session) IsIdle(timeout time.Duration) bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	return time.Since(s.lastRequest) > timeout
}

var (
	sessions   = make(map[uuid.UUID]*Session)
	sessionsMu sync.Mutex
)

// GetOrCreateSession returns a context for FFmpeg.
// The boolean is true if a NEW session was created, false if one already exists.
func GetOrCreateSession(vaultItemId uuid.UUID) (context.Context, bool) {
	sessionsMu.Lock()
	defer sessionsMu.Unlock()

	if s, exists := sessions[vaultItemId]; exists {
		s.Touch()
		return nil, false
	}

	ctx, cancel := context.WithCancel(context.Background())
	s := &Session{
		VaultItemId: vaultItemId,
		Cancel:      cancel,
		lastRequest: time.Now(),
	}
	sessions[vaultItemId] = s
	return ctx, true
}

func RemoveSession(vaultItemId uuid.UUID) {
	sessionsMu.Lock()
	defer sessionsMu.Unlock()

	if s, exists := sessions[vaultItemId]; exists {
		s.Cancel() // Terminate FFmpeg!
		delete(sessions, vaultItemId)
	}
}

func TouchSession(vaultItemId uuid.UUID) {
	sessionsMu.Lock()
	defer sessionsMu.Unlock()
	if s, exists := sessions[vaultItemId]; exists {
		s.Touch()
	}
}

// TODO: Replace with go-cron job
func init() {
	// Background reaper to kill FFmpeg processes idle for > 2 minutes
	go func() {
		ticker := time.NewTicker(30 * time.Second)
		for range ticker.C {
			sessionsMu.Lock()
			for id, s := range sessions {
				if s.IsIdle(2 * time.Minute) {
					s.Cancel()
					delete(sessions, id)
				}
			}
			sessionsMu.Unlock()
		}
	}()
}
