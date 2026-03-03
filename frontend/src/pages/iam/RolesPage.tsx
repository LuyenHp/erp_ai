import React, { useEffect, useState } from 'react';
import {
    Plus,
    Shield,
    Settings,
    Trash2,
    CheckCircle2,
    XCircle,
    AlertCircle,
    Loader2,
    Lock,
    Search
} from 'lucide-react';
import api from '../../services/api';

interface Role {
    id: string;
    name: string;
    code: string;
    description: string;
    is_system: boolean;
    permissions?: string[];
}

interface Permission {
    id: string;
    name: string;
    code: string;
    description: string;
    resource_type: string;
}

const RolesPage: React.FC = () => {
    const [roles, setRoles] = useState<Role[]>([]);
    const [permissions, setPermissions] = useState<Permission[]>([]);
    const [loading, setLoading] = useState(true);
    const [selectedRole, setSelectedRole] = useState<Role | null>(null);
    const [searchTerm, setSearchTerm] = useState('');

    const fetchData = async () => {
        setLoading(true);
        try {
            const [{ data: rolesData }, { data: permsData }] = await Promise.all([
                api.get('/api/roles'),
                api.get('/api/permissions')
            ]);
            setRoles(rolesData);
            setPermissions(permsData);
            if (rolesData.length > 0) setSelectedRole(rolesData[0]);
        } catch (err) {
            console.error('Error fetching IAM data:', err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchData();
    }, []);

    const filteredRoles = roles.filter(r =>
        r.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        r.code.toLowerCase().includes(searchTerm.toLowerCase())
    );

    return (
        <div className="p-8 animate-fade-in">
            <div className="flex justify-between items-center mb-8">
                <div>
                    <h1 className="heading" style={{ fontSize: '1.75rem' }}>Vai trò & Quyền hạn</h1>
                    <p className="text-dim">Định nghĩa các nhóm quyền và quản lý truy cập hệ thống.</p>
                </div>
                <button className="primary-btn">
                    <Plus size={18} />
                    <span>Tạo Vai trò</span>
                </button>
            </div>

            <div className="grid grid-cols-12 gap-6">
                {/* Left Sidebar: Role List */}
                <div className="col-span-4 flex flex-col gap-4">
                    <div className="glass-card p-2 flex items-center gap-2 px-3">
                        <Search size={16} className="text-muted" />
                        <input
                            type="text"
                            placeholder="Tìm kiếm vai trò..."
                            className="bg-transparent border-none outline-none text-sm w-full py-1"
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>

                    <div className="flex flex-col gap-2 overflow-y-auto" style={{ maxHeight: 'calc(100vh - 300px)' }}>
                        {loading ? (
                            <div className="p-8 text-center"><Loader2 className="animate-spin mx-auto text-primary" /></div>
                        ) : filteredRoles.map(role => (
                            <div
                                key={role.id}
                                onClick={() => setSelectedRole(role)}
                                className={`glass-card p-4 cursor-pointer transition-all border ${selectedRole?.id === role.id ? 'border-primary ring-1 ring-primary-glow' : 'border-transparent hover:border-border'}`}
                            >
                                <div className="flex justify-between items-start">
                                    <div>
                                        <div className="flex items-center gap-2">
                                            <span className="font-semibold">{role.name}</span>
                                            {role.is_system && <Lock size={12} className="text-warning" />}
                                        </div>
                                        <span className="text-xs text-muted block mt-1">{role.code}</span>
                                    </div>
                                    <Shield size={16} className={selectedRole?.id === role.id ? 'text-primary' : 'text-muted'} />
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                {/* Right Area: Permissions Matrix */}
                <div className="col-span-8">
                    {selectedRole ? (
                        <div className="glass-card p-6 min-h-full">
                            <div className="flex justify-between items-start mb-6">
                                <div>
                                    <h2 className="heading text-xl">{selectedRole.name}</h2>
                                    <p className="text-sm text-muted">{selectedRole.description || 'Không có mô tả.'}</p>
                                </div>
                                <div className="flex gap-2">
                                    <button className="icon-btn-sm hover:text-error">
                                        <Trash2 size={16} />
                                    </button>
                                    <button className="icon-btn-sm">
                                        <Settings size={16} />
                                    </button>
                                </div>
                            </div>

                            <div className="permission-grid">
                                <div className="grid-header grid grid-cols-12 pb-3 border-bottom border-border mb-4">
                                    <div className="col-span-6 text-xs text-muted font-bold uppercase tracking-wider">Tên quyền</div>
                                    <div className="col-span-4 text-xs text-muted font-bold uppercase tracking-wider">Mã hiệu</div>
                                    <div className="col-span-2 text-xs text-center text-muted font-bold uppercase tracking-wider">Trạng thái</div>
                                </div>

                                <div className="grid-body flex flex-col gap-1">
                                    {permissions.map(perm => (
                                        <div key={perm.id} className="grid grid-cols-12 py-3 px-2 hover:bg-white/5 rounded-lg transition-all items-center">
                                            <div className="col-span-6">
                                                <span className="text-sm font-medium">{perm.name}</span>
                                                <p className="text-[10px] text-muted">{perm.resource_type}</p>
                                            </div>
                                            <div className="col-span-4">
                                                <code className="text-[11px] bg-black/40 px-2 py-0.5 rounded text-primary">{perm.code}</code>
                                            </div>
                                            <div className="col-span-2 flex justify-center">
                                                <button className="p-1 hover:scale-110 transition-transform">
                                                    <CheckCircle2 size={20} className="text-success" />
                                                </button>
                                            </div>
                                        </div>
                                    ))}

                                    {permissions.length === 0 && (
                                        <div className="p-8 text-center text-muted">
                                            <AlertCircle className="mx-auto mb-2 opacity-20" size={32} />
                                            <p>Chưa có quyền hạn nào được cấu hình.</p>
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div className="mt-8 pt-6 border-top border-border">
                                <button className="primary-btn w-full">Lưu thay đổi</button>
                            </div>
                        </div>
                    ) : (
                        <div className="glass-card p-12 flex flex-col items-center justify-center opacity-40">
                            <Shield size={48} className="mb-4" />
                            <p>Chọn một vai trò để cấu hình quyền truy cập.</p>
                        </div>
                    )}
                </div>
            </div>

            <style>{`
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

        .icon-btn-sm {
          background: rgba(255,255,255,0.05);
          border: 1px solid var(--border);
          color: var(--text-dim);
          width: 32px;
          height: 32px;
          border-radius: 0.5rem;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          transition: var(--transition-smooth);
        }

        .icon-btn-sm:hover { color: white; border-color: var(--border-bright); }

        .permission-grid {
          margin-top: 1rem;
        }

        .border-bottom { border-bottom: 1px solid var(--border); }
        .border-top { border-top: 1px solid var(--border); }
      `}</style>
        </div>
    );
};

export default RolesPage;
