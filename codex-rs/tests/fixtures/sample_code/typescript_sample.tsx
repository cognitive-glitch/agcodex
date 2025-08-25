// Sample TypeScript React component for testing AST parsing

import React, { useState, useEffect, useCallback } from 'react';

interface User {
  id: number;
  name: string;
  email: string;
  avatar?: string;
}

interface UserListProps {
  users: User[];
  onUserSelect?: (user: User) => void;
  loading?: boolean;
  className?: string;
}

interface UserCardProps {
  user: User;
  selected?: boolean;
  onClick?: () => void;
}

const UserCard: React.FC<UserCardProps> = ({ user, selected = false, onClick }) => {
  return (
    <div
      className={`user-card ${selected ? 'selected' : ''}`}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          onClick?.();
        }
      }}
    >
      <div className="user-avatar">
        {user.avatar ? (
          <img src={user.avatar} alt={`${user.name}'s avatar`} />
        ) : (
          <div className="avatar-placeholder">
            {user.name.charAt(0).toUpperCase()}
          </div>
        )}
      </div>
      <div className="user-info">
        <h3 className="user-name">{user.name}</h3>
        <p className="user-email">{user.email}</p>
      </div>
    </div>
  );
};

const UserList: React.FC<UserListProps> = ({
  users,
  onUserSelect,
  loading = false,
  className = ''
}) => {
  const [selectedUserId, setSelectedUserId] = useState<number | null>(null);
  const [searchTerm, setSearchTerm] = useState('');

  const filteredUsers = React.useMemo(() => {
    if (!searchTerm) return users;
    
    return users.filter(user =>
      user.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      user.email.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }, [users, searchTerm]);

  const handleUserClick = useCallback((user: User) => {
    setSelectedUserId(user.id);
    onUserSelect?.(user);
  }, [onUserSelect]);

  useEffect(() => {
    if (selectedUserId && !users.find(u => u.id === selectedUserId)) {
      setSelectedUserId(null);
    }
  }, [users, selectedUserId]);

  if (loading) {
    return (
      <div className={`user-list loading ${className}`}>
        <div className="loading-spinner">Loading users...</div>
      </div>
    );
  }

  return (
    <div className={`user-list ${className}`}>
      <div className="user-list-header">
        <h2>Users ({users.length})</h2>
        <input
          type="text"
          placeholder="Search users..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="search-input"
        />
      </div>

      <div className="user-list-content">
        {filteredUsers.length === 0 ? (
          <div className="empty-state">
            {searchTerm ? 'No users match your search.' : 'No users found.'}
          </div>
        ) : (
          filteredUsers.map(user => (
            <UserCard
              key={user.id}
              user={user}
              selected={user.id === selectedUserId}
              onClick={() => handleUserClick(user)}
            />
          ))
        )}
      </div>
    </div>
  );
};

// Custom hook for managing user data
export const useUsers = () => {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchUsers = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      const mockUsers: User[] = [
        { id: 1, name: 'Alice Johnson', email: 'alice@example.com' },
        { id: 2, name: 'Bob Smith', email: 'bob@example.com' },
        { id: 3, name: 'Carol Davis', email: 'carol@example.com' },
      ];
      
      setUsers(mockUsers);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch users');
    } finally {
      setLoading(false);
    }
  }, []);

  const addUser = useCallback((user: Omit<User, 'id'>) => {
    const newUser: User = {
      ...user,
      id: Math.max(...users.map(u => u.id), 0) + 1,
    };
    setUsers(prev => [...prev, newUser]);
    return newUser;
  }, [users]);

  const removeUser = useCallback((userId: number) => {
    setUsers(prev => prev.filter(user => user.id !== userId));
  }, []);

  const updateUser = useCallback((userId: number, updates: Partial<User>) => {
    setUsers(prev =>
      prev.map(user =>
        user.id === userId ? { ...user, ...updates } : user
      )
    );
  }, []);

  useEffect(() => {
    fetchUsers();
  }, [fetchUsers]);

  return {
    users,
    loading,
    error,
    refetch: fetchUsers,
    addUser,
    removeUser,
    updateUser,
  };
};

// Main component that uses the hook and renders the list
export const UserManagementPage: React.FC = () => {
  const { users, loading, error, refetch, addUser } = useUsers();
  const [selectedUser, setSelectedUser] = useState<User | null>(null);

  const handleAddUser = () => {
    const name = prompt('Enter user name:');
    const email = prompt('Enter user email:');
    
    if (name && email) {
      addUser({ name, email });
    }
  };

  if (error) {
    return (
      <div className="error-state">
        <h2>Error loading users</h2>
        <p>{error}</p>
        <button onClick={refetch}>Try again</button>
      </div>
    );
  }

  return (
    <div className="user-management-page">
      <header className="page-header">
        <h1>User Management</h1>
        <button onClick={handleAddUser} className="add-user-btn">
          Add User
        </button>
      </header>

      <div className="page-content">
        <div className="user-list-section">
          <UserList
            users={users}
            loading={loading}
            onUserSelect={setSelectedUser}
          />
        </div>

        {selectedUser && (
          <div className="user-details-section">
            <h3>Selected User</h3>
            <div className="user-details">
              <p><strong>Name:</strong> {selectedUser.name}</p>
              <p><strong>Email:</strong> {selectedUser.email}</p>
              <p><strong>ID:</strong> {selectedUser.id}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default UserManagementPage;