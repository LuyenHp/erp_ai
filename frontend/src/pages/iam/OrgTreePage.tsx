import React, { useEffect, useState } from 'react';
import {
    Plus,
    ChevronRight,
    ChevronDown,
    Building2,
    MoreVertical,
    Briefcase,
    MapPin,
    RefreshCcw,
    Loader2
} from 'lucide-react';
import api from '../../services/api';

interface Department {
    id: string;
    name: string;
    code: string;
    parent_id: string | null;
    path: string;
    level: number;
    description: string;
    children?: Department[];
}

const OrgTreePage: React.FC = () => {
    const [tree, setTree] = useState<Department[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const fetchTree = async () => {
        setLoading(true);
        try {
            const { data } = await api.get('/api/departments/tree');
            setTree(data);
        } catch (err) {
            setError('Không thể tải sơ đồ tổ chức.');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchTree();
    }, []);

    return (
        <div className="org-tree-container p-8 animate-fade-in">
            {/* Header Area */}
            <div className="flex justify-between items-center mb-8">
                <div>
                    <h1 className="heading" style={{ fontSize: '1.75rem' }}>Sơ đồ tổ chức</h1>
                    <p className="text-dim">Quản lý các phòng ban, chi nhánh và cơ cấu phân cấp.</p>
                </div>
                <div className="flex gap-3">
                    <button className="icon-btn" onClick={fetchTree} disabled={loading}>
                        {loading ? <Loader2 className="animate-spin" size={18} /> : <RefreshCcw size={18} />}
                    </button>
                    <button className="primary-btn">
                        <Plus size={18} />
                        <span>Thêm đơn vị</span>
                    </button>
                </div>
            </div>

            {/* Stats Quick View */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
                <div className="glass-card p-4 flex items-center gap-4">
                    <div className="icon-box bg-primary"><Building2 size={20} /></div>
                    <div>
                        <span className="block text-xs text-muted">Tổng phòng ban</span>
                        <span className="heading">12</span>
                    </div>
                </div>
                <div className="glass-card p-4 flex items-center gap-4">
                    <div className="icon-box bg-accent"><MapPin size={20} /></div>
                    <div>
                        <span className="block text-xs text-muted">Chi nhánh</span>
                        <span className="heading">3</span>
                    </div>
                </div>
            </div>

            {/* The Recursive Tree View */}
            <div className="glass-card p-6">
                {loading && <div className="p-12 text-center text-muted">Đang phân tích cấu trúc...</div>}
                {error && <div className="p-12 text-center text-error">{error}</div>}
                {!loading && tree.length === 0 && (
                    <div className="p-12 text-center text-muted border-dashed border-2 rounded-xl">
                        Chưa có cấu trúc tổ chức. <br />
                        Bấm "Thêm đơn vị" để bắt đầu.
                    </div>
                )}

                <div className="tree-root">
                    {tree.map(node => (
                        <TreeNode key={node.id} node={node} />
                    ))}
                </div>
            </div>

            <style>{`
        .org-tree-container {
        }

        .icon-btn {
          background: var(--bg-card);
          border: 1px solid var(--border);
          color: white;
          width: 40px;
          height: 40px;
          border-radius: 0.75rem;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          transition: var(--transition-smooth);
        }

        .icon-btn:hover { border-color: var(--border-bright); background: rgba(255,255,255,0.05); }

        .primary-btn {
          background: hsl(var(--primary));
          color: white;
          border: none;
          padding: 0 1.25rem;
          height: 40px;
          border-radius: 0.75rem;
          font-weight: 600;
          font-size: 0.875rem;
          display: flex;
          align-items: center;
          gap: 0.5rem;
          cursor: pointer;
          transition: var(--transition-smooth);
        }

        .primary-btn:hover { transform: translateY(-2px); box-shadow: 0 10px 20px -5px hsl(var(--primary) / 0.3); }

        .icon-box {
          width: 40px;
          height: 40px;
          border-radius: 0.75rem;
          display: flex;
          align-items: center;
          justify-content: center;
          background: rgba(255,255,255,0.05);
          color: white;
        }

        .icon-box.bg-primary { color: hsl(var(--primary)); }
        .icon-box.bg-accent { color: hsl(var(--accent-cyan)); }

        .tree-root {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }

        .tree-node-item {
          user-select: none;
        }

        .node-row {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 0.75rem 1rem;
          border-radius: 0.75rem;
          background: rgba(255,255,255,0.02);
          border: 1px solid transparent;
          transition: var(--transition-smooth);
          cursor: pointer;
        }

        .node-row:hover {
          background: rgba(255,255,255,0.05);
          border-color: var(--border-bright);
        }

        .node-expander {
          color: hsl(var(--text-muted));
          display: flex;
          align-items: center;
          justify-content: center;
          width: 20px;
        }

        .node-icon {
          color: hsl(var(--primary));
        }

        .node-info {
          flex: 1;
          display: flex;
          align-items: center;
          gap: 1rem;
        }

        .node-name { font-weight: 500; font-size: 0.9375rem; }
        .node-code { 
          font-size: 0.6875rem; 
          background: var(--bg-deep); 
          padding: 0.125rem 0.5rem; 
          border-radius: 4px; 
          color: hsl(var(--text-dim));
          font-family: monospace;
          letter-spacing: 0.05em;
        }

        .node-actions {
          opacity: 0;
          transition: var(--transition-smooth);
          color: hsl(var(--text-muted));
        }

        .node-row:hover .node-actions { opacity: 1; }

        .tree-children {
          padding-left: 2rem;
          margin-top: 0.5rem;
          border-left: 1px solid var(--border);
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }
      `}</style>
        </div>
    );
};

// Recursive Node Component
const TreeNode = ({ node }: { node: Department }) => {
    const [expanded, setExpanded] = useState(true);
    const hasChildren = node.children && node.children.length > 0;

    return (
        <div className="tree-node-item">
            <div className="node-row" onClick={() => setExpanded(!expanded)}>
                <div className="node-expander">
                    {hasChildren && (expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />)}
                </div>
                <div className="node-icon">
                    {node.level === 0 ? <Building2 size={18} /> : <Briefcase size={16} />}
                </div>
                <div className="node-info">
                    <span className="node-name">{node.name}</span>
                    <span className="node-code">{node.code}</span>
                </div>
                <div className="node-actions">
                    <MoreVertical size={16} />
                </div>
            </div>

            {hasChildren && expanded && (
                <div className="tree-children">
                    {node.children!.map(child => (
                        <TreeNode key={child.id} node={child} />
                    ))}
                </div>
            )}
        </div>
    );
};

export default OrgTreePage;
