# Sample Python code for testing AST parsing and compression

from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from datetime import datetime
import asyncio
import json

@dataclass
class Task:
    """A simple task with ID, title, and status."""
    id: str
    title: str
    completed: bool = False
    created_at: datetime = field(default_factory=datetime.now)
    tags: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)

    def mark_completed(self) -> None:
        """Mark the task as completed."""
        self.completed = True

    def add_tag(self, tag: str) -> None:
        """Add a tag to the task."""
        if tag not in self.tags:
            self.tags.append(tag)

    def remove_tag(self, tag: str) -> None:
        """Remove a tag from the task."""
        if tag in self.tags:
            self.tags.remove(tag)

    def to_dict(self) -> Dict[str, Any]:
        """Convert task to dictionary."""
        return {
            'id': self.id,
            'title': self.title,
            'completed': self.completed,
            'created_at': self.created_at.isoformat(),
            'tags': self.tags,
            'metadata': self.metadata
        }

class TaskManager:
    """Manages a collection of tasks."""
    
    def __init__(self):
        self.tasks: Dict[str, Task] = {}
    
    def add_task(self, task: Task) -> None:
        """Add a task to the manager."""
        self.tasks[task.id] = task
    
    def get_task(self, task_id: str) -> Optional[Task]:
        """Get a task by ID."""
        return self.tasks.get(task_id)
    
    def remove_task(self, task_id: str) -> bool:
        """Remove a task by ID. Returns True if task was found and removed."""
        if task_id in self.tasks:
            del self.tasks[task_id]
            return True
        return False
    
    def list_tasks(self, completed: Optional[bool] = None) -> List[Task]:
        """List all tasks, optionally filtered by completion status."""
        tasks = list(self.tasks.values())
        
        if completed is not None:
            tasks = [task for task in tasks if task.completed == completed]
        
        return sorted(tasks, key=lambda t: t.created_at)
    
    def find_tasks_by_tag(self, tag: str) -> List[Task]:
        """Find all tasks that have the specified tag."""
        return [task for task in self.tasks.values() if tag in task.tags]
    
    def mark_task_completed(self, task_id: str) -> bool:
        """Mark a task as completed. Returns True if task was found."""
        task = self.get_task(task_id)
        if task:
            task.mark_completed()
            return True
        return False
    
    def get_stats(self) -> Dict[str, int]:
        """Get statistics about tasks."""
        total = len(self.tasks)
        completed = sum(1 for task in self.tasks.values() if task.completed)
        pending = total - completed
        
        return {
            'total': total,
            'completed': completed,
            'pending': pending
        }
    
    def export_to_json(self) -> str:
        """Export all tasks to JSON string."""
        task_dicts = [task.to_dict() for task in self.tasks.values()]
        return json.dumps(task_dicts, indent=2)
    
    async def save_to_file(self, filename: str) -> None:
        """Asynchronously save tasks to a file."""
        data = self.export_to_json()
        
        # Simulate async file I/O
        await asyncio.sleep(0.01)
        
        with open(filename, 'w') as f:
            f.write(data)

async def example_usage():
    """Example usage of the task management system."""
    manager = TaskManager()
    
    # Create some tasks
    task1 = Task("1", "Learn Python", tags=["learning", "programming"])
    task2 = Task("2", "Write tests", tags=["programming", "testing"])
    task3 = Task("3", "Deploy application", tags=["devops"])
    
    # Add tasks to manager
    for task in [task1, task2, task3]:
        manager.add_task(task)
    
    # Mark one task as completed
    manager.mark_task_completed("1")
    
    # List pending tasks
    pending = manager.list_tasks(completed=False)
    print(f"Pending tasks: {[task.title for task in pending]}")
    
    # Find programming tasks
    programming_tasks = manager.find_tasks_by_tag("programming")
    print(f"Programming tasks: {[task.title for task in programming_tasks]}")
    
    # Get stats
    stats = manager.get_stats()
    print(f"Task statistics: {stats}")
    
    # Save to file
    await manager.save_to_file("tasks.json")

if __name__ == "__main__":
    asyncio.run(example_usage())