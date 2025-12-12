package auth

import "errors"

var (
	ErrUserNotFound      = errors.New("user not found")
	ErrUserAlreadyExists = errors.New("user already exists")
)

// User represents a system user
type User struct {
	ID           string `json:"id"`
	Username     string `json:"username"`
	PasswordHash string `json:"password_hash"`
	Role         string `json:"role"`
}

// UserStore defines the interface for user persistence
type UserStore interface {
	// GetUser retrieves one user by username
	GetUser(username string) (*User, error)
	// SaveUser Creates or updates a user
	SaveUser(user *User) error
	// ListUsers returns all users
	ListUsers() ([]*User, error)
}
