package auth

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/lumadb/cluster/pkg/cluster"
	"go.uber.org/zap"
)

var (
	ErrInvalidToken = errors.New("invalid token")
	ErrExpiredToken = errors.New("expired token")
)

type Action string

const (
	ActionRead   Action = "read"
	ActionWrite  Action = "write"
	ActionDelete Action = "delete"
	ActionManage Action = "manage"
)

type Claims struct {
	UserID string `json:"user_id"`
	Role   string `json:"role"`
	jwt.RegisteredClaims
}

type AuthEngine struct {
	node        *cluster.Node
	logger      *zap.Logger
	secretKey   []byte
	store       UserStore
	permissions map[string]map[Action]bool // role -> action -> allowed
}

func NewAuthEngine(node *cluster.Node, logger *zap.Logger) (*AuthEngine, error) {
	// Initialize File Store (MVP)
	store, err := NewFileUserStore("users.json")
	if err != nil {
		return nil, err
	}

	// Create default admin if not exists
	if _, err := store.GetUser("admin"); err == ErrUserNotFound {
		store.SaveUser(&User{
			Username:     "admin",
			PasswordHash: "password", // In production: bcrypt
			Role:         "admin",
		})
	}

	e := &AuthEngine{
		node:        node,
		logger:      logger,
		store:       store,
		secretKey:   []byte("luma-super-secret-key-change-me"),
		permissions: make(map[string]map[Action]bool),
	}

	// Setup Default Roles (MVP)
	e.permissions["admin"] = map[Action]bool{
		ActionRead:   true,
		ActionWrite:  true,
		ActionDelete: true,
		ActionManage: true,
	}
	e.permissions["viewer"] = map[Action]bool{
		ActionRead: true,
	}

	return e, nil
}

func (e *AuthEngine) Start() error {
	e.logger.Info("Auth Engine started")
	return nil
}

// GenerateToken creates a new JWT for a user
func (e *AuthEngine) GenerateToken(username, password string) (string, error) {
	user, err := e.store.GetUser(username)
	if err != nil {
		return "", ErrUserNotFound
	}

	// In production: Verify password hash
	if user.PasswordHash != password {
		return "", errors.New("invalid password")
	}

	expirationTime := time.Now().Add(24 * time.Hour)
	claims := &Claims{
		UserID: user.ID,
		Role:   user.Role,
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(expirationTime),
			Issuer:    "luma-platform",
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString(e.secretKey)
}

// Register creates a new user
func (e *AuthEngine) Register(username, password, role string) error {
	if _, err := e.store.GetUser(username); err == nil {
		return ErrUserAlreadyExists
	}

	return e.store.SaveUser(&User{
		ID:           username, // Simple ID for MVP
		Username:     username,
		PasswordHash: password, // In production: bcrypt
		Role:         role,
	})
}

// ValidateToken parses and validates a JWT
func (e *AuthEngine) ValidateToken(tokenString string) (*Claims, error) {
	claims := &Claims{}

	token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
		return e.secretKey, nil
	})

	if err != nil {
		if err == jwt.ErrTokenExpired {
			return nil, ErrExpiredToken
		}
		return nil, err
	}

	if !token.Valid {
		return nil, ErrInvalidToken
	}

	return claims, nil
}

// IsAuthorized checks if a role can perform an action
// Support for granular permissions: "read:collection", "write:users"
func (e *AuthEngine) IsAuthorized(role string, action Action) bool {
	// 1. Check coarse-grained permissions (e.g. "read" allows everything if map says true)
	if roleMap, ok := e.permissions[role]; ok {
		if allowed, ok := roleMap[action]; ok && allowed {
			return true
		}
	}

	// 2. Granular checks (MVP: Admin has everything)
	if role == "admin" {
		return true
	}

	// 3. Handle Resource-scoped permissions (e.g. action="read:users")
	// If the user has "read" permission, they might be allowed "read:*" implicitly?
	// Or we check logic: if permission is "read:users", role needs "read:users"
	// For MVP: Check strict equality in map first (done above).
	// Then check if role has "manage" (superuser for that resource type?)

	// Example: role "viewer" has ActionRead.
	// If request is "read:users", does ActionRead cover it?
	// For now, let's assume ActionRead implies read access to standard collections.
	// But "read:users" might be special.
	// Let's keep it simple: strict check passed above. If not found, return false.
	// UNLESS admin.

	return false
}
