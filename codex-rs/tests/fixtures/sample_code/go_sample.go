// Sample Go code for testing AST parsing and compression

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"strconv"
	"sync"
	"time"

	"github.com/gorilla/mux"
)

// User represents a user in the system
type User struct {
	ID       int       `json:"id" db:"id"`
	Name     string    `json:"name" db:"name"`
	Email    string    `json:"email" db:"email"`
	Active   bool      `json:"active" db:"active"`
	Created  time.Time `json:"created" db:"created_at"`
	Updated  time.Time `json:"updated" db:"updated_at"`
}

// UserService defines the interface for user operations
type UserService interface {
	CreateUser(ctx context.Context, user *User) error
	GetUser(ctx context.Context, id int) (*User, error)
	UpdateUser(ctx context.Context, user *User) error
	DeleteUser(ctx context.Context, id int) error
	ListUsers(ctx context.Context, limit, offset int) ([]*User, error)
}

// userService implements UserService
type userService struct {
	mu    sync.RWMutex
	users map[int]*User
	nextID int
}

// NewUserService creates a new user service
func NewUserService() UserService {
	return &userService{
		users:  make(map[int]*User),
		nextID: 1,
	}
}

func (s *userService) CreateUser(ctx context.Context, user *User) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	user.ID = s.nextID
	s.nextID++
	user.Created = time.Now()
	user.Updated = time.Now()
	user.Active = true

	s.users[user.ID] = user
	return nil
}

func (s *userService) GetUser(ctx context.Context, id int) (*User, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	user, exists := s.users[id]
	if !exists {
		return nil, fmt.Errorf("user with id %d not found", id)
	}

	// Return a copy to avoid concurrent modification
	userCopy := *user
	return &userCopy, nil
}

func (s *userService) UpdateUser(ctx context.Context, user *User) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	existing, exists := s.users[user.ID]
	if !exists {
		return fmt.Errorf("user with id %d not found", user.ID)
	}

	// Update fields
	existing.Name = user.Name
	existing.Email = user.Email
	existing.Active = user.Active
	existing.Updated = time.Now()

	return nil
}

func (s *userService) DeleteUser(ctx context.Context, id int) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if _, exists := s.users[id]; !exists {
		return fmt.Errorf("user with id %d not found", id)
	}

	delete(s.users, id)
	return nil
}

func (s *userService) ListUsers(ctx context.Context, limit, offset int) ([]*User, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	users := make([]*User, 0, len(s.users))
	for _, user := range s.users {
		users = append(users, &User{
			ID:      user.ID,
			Name:    user.Name,
			Email:   user.Email,
			Active:  user.Active,
			Created: user.Created,
			Updated: user.Updated,
		})
	}

	// Simple pagination
	start := offset
	if start > len(users) {
		return []*User{}, nil
	}

	end := start + limit
	if end > len(users) {
		end = len(users)
	}

	return users[start:end], nil
}

// UserHandler handles HTTP requests for user operations
type UserHandler struct {
	service UserService
}

// NewUserHandler creates a new user handler
func NewUserHandler(service UserService) *UserHandler {
	return &UserHandler{service: service}
}

func (h *UserHandler) CreateUser(w http.ResponseWriter, r *http.Request) {
	var user User
	if err := json.NewDecoder(r.Body).Decode(&user); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}

	if err := h.service.CreateUser(r.Context(), &user); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(&user)
}

func (h *UserHandler) GetUser(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	idStr := vars["id"]

	id, err := strconv.Atoi(idStr)
	if err != nil {
		http.Error(w, "Invalid user ID", http.StatusBadRequest)
		return
	}

	user, err := h.service.GetUser(r.Context(), id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(user)
}

func (h *UserHandler) UpdateUser(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	idStr := vars["id"]

	id, err := strconv.Atoi(idStr)
	if err != nil {
		http.Error(w, "Invalid user ID", http.StatusBadRequest)
		return
	}

	var user User
	if err := json.NewDecoder(r.Body).Decode(&user); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}

	user.ID = id
	if err := h.service.UpdateUser(r.Context(), &user); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(&user)
}

func (h *UserHandler) DeleteUser(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	idStr := vars["id"]

	id, err := strconv.Atoi(idStr)
	if err != nil {
		http.Error(w, "Invalid user ID", http.StatusBadRequest)
		return
	}

	if err := h.service.DeleteUser(r.Context(), id); err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (h *UserHandler) ListUsers(w http.ResponseWriter, r *http.Request) {
	limitStr := r.URL.Query().Get("limit")
	offsetStr := r.URL.Query().Get("offset")

	limit := 10 // default
	if limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil && l > 0 {
			limit = l
		}
	}

	offset := 0 // default
	if offsetStr != "" {
		if o, err := strconv.Atoi(offsetStr); err == nil && o >= 0 {
			offset = o
		}
	}

	users, err := h.service.ListUsers(r.Context(), limit, offset)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(users)
}

// setupRoutes configures the HTTP routes
func setupRoutes(handler *UserHandler) *mux.Router {
	r := mux.NewRouter()

	api := r.PathPrefix("/api/v1").Subrouter()
	api.HandleFunc("/users", handler.CreateUser).Methods("POST")
	api.HandleFunc("/users", handler.ListUsers).Methods("GET")
	api.HandleFunc("/users/{id:[0-9]+}", handler.GetUser).Methods("GET")
	api.HandleFunc("/users/{id:[0-9]+}", handler.UpdateUser).Methods("PUT")
	api.HandleFunc("/users/{id:[0-9]+}", handler.DeleteUser).Methods("DELETE")

	return r
}

// middleware for logging requests
func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		log.Printf("%s %s %v", r.Method, r.URL.Path, time.Since(start))
	})
}

func main() {
	service := NewUserService()
	handler := NewUserHandler(service)
	router := setupRoutes(handler)

	// Add middleware
	router.Use(loggingMiddleware)

	srv := &http.Server{
		Addr:         ":8080",
		Handler:      router,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	log.Println("Server starting on :8080")
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatal("Server failed to start:", err)
	}
}