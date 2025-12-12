package auth

import (
	"encoding/json"
	"os"
	"sync"
)

// FileUserStore is a simple JSON-file based user store
type FileUserStore struct {
	path  string
	mu    sync.RWMutex
	users map[string]*User // username -> User
}

// NewFileUserStore creates a new file-based user store
func NewFileUserStore(path string) (*FileUserStore, error) {
	store := &FileUserStore{
		path:  path,
		users: make(map[string]*User),
	}

	// Load existing
	if err := store.load(); err != nil {
		return nil, err
	}

	return store, nil
}

func (s *FileUserStore) load() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	data, err := os.ReadFile(s.path)
	if os.IsNotExist(err) {
		return nil // New store
	}
	if err != nil {
		return err
	}

	if len(data) == 0 {
		return nil
	}

	var usersList []*User
	if err := json.Unmarshal(data, &usersList); err != nil {
		return err
	}

	for _, u := range usersList {
		s.users[u.Username] = u
	}
	return nil
}

func (s *FileUserStore) save() error {
	// Must hold lock
	var usersList []*User
	for _, u := range s.users {
		usersList = append(usersList, u)
	}

	data, err := json.MarshalIndent(usersList, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(s.path, data, 0644)
}

func (s *FileUserStore) GetUser(username string) (*User, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	u, ok := s.users[username]
	if !ok {
		return nil, ErrUserNotFound
	}
	return u, nil
}

func (s *FileUserStore) SaveUser(user *User) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.users[user.Username] = user
	return s.save()
}

func (s *FileUserStore) ListUsers() ([]*User, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var list []*User
	for _, u := range s.users {
		list = append(list, u)
	}
	return list, nil
}
