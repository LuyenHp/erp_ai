import React, { useState, useEffect } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import {
  BarChart3,
  Users,
  Settings,
  Search,
  Bell,
  LogOut,
  ChevronRight,
  ShieldCheck,
  Zap,
  Boxes,
  ClipboardList,
  Loader2,
  Send
} from 'lucide-react';
import { clsx } from 'clsx';
import api from '../services/api';

interface MainLayoutProps {
  children: React.ReactNode;
}

const MainLayout: React.FC<MainLayoutProps> = ({ children }) => {
  const [commandBarOpen, setCommandBarOpen] = useState(false);
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiResponse, setAiResponse] = useState<{ message: string; action?: any } | null>(null);
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const user = JSON.parse(localStorage.getItem('user') || '{}');

  // Command Bar Shortcut (Ctrl+K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        setAiPrompt('');
        setAiResponse(null);
        setCommandBarOpen(true);
      }
      if (e.key === 'Escape') setCommandBarOpen(false);
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const handleSendCommand = async () => {
    if (!aiPrompt.trim() || loading) return;
    setLoading(true);
    try {
      const { data } = await api.post('/api/ai/command', { prompt: aiPrompt });
      setAiResponse(data);

      // Execute AI Action if provided
      if (data.action) {
        const { type, payload } = data.action;
        if (type === 'navigate') {
          navigate(payload.path);
          setCommandBarOpen(false);
        } else if (type === 'create_department') {
          // Refresh current page if on org tree
          if (window.location.pathname === '/iam/org') {
            window.location.reload();
          }
        }
      }
    } catch (err) {
      setAiResponse({ message: "AI Brain đang gặp lỗi kết nối. Vui lòng thử lại sau.", action: null });
    } finally {
      setLoading(false);
    }
  };

  const handleLogout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    navigate('/login');
    window.location.reload();
  };

  return (
    <div className="layout-container">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="logo-box">
            <Zap className="text-primary-glow" />
            <span className="heading">ERP AI</span>
          </div>
        </div>

        <nav className="sidebar-nav">
          <div className="nav-section">
            <span className="section-label">General</span>
            <SidebarLink to="/" icon={<BarChart3 size={20} />} label="Tổng quan" />
            <SidebarLink to="/audit" icon={<ClipboardList size={20} />} label="Audit Log" />
          </div>

          <div className="nav-section">
            <span className="section-label">Identity & Access</span>
            <SidebarLink to="/iam/org" icon={<Boxes size={20} />} label="Sơ đồ tổ chức" />
            <SidebarLink to="/iam/roles" icon={<ShieldCheck size={20} />} label="Phân quyền" />
            <SidebarLink to="/iam/approvals" icon={<Bell size={20} />} label="Duyệt yêu cầu" />
          </div>

          <div className="nav-section">
            <span className="section-label">HR & People</span>
            <SidebarLink to="/employees" icon={<Users size={20} />} label="Nhân sự" />
          </div>
        </nav>

        <div className="sidebar-footer">
          <div className="user-info">
            <div className="user-avatar">
              {user.full_name?.charAt(0) || 'U'}
            </div>
            <div className="user-meta">
              <span className="user-name">{user.full_name}</span>
              <span className="user-role">{user.is_superadmin ? 'Super Admin' : 'Staff'}</span>
            </div>
          </div>
          <button className="logout-btn" onClick={handleLogout}>
            <LogOut size={18} />
          </button>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="main-viewport">
        {/* Top Header */}
        <header className="top-header">
          <div className="header-left">
            {/* Can add breadcrumbs or page title here later */}
          </div>

          <div className="header-center">
            <div className="search-trigger" onClick={() => setCommandBarOpen(true)}>
              <Search size={16} className="text-muted" />
              <span>Search or command (Ctrl+K)</span>
            </div>
          </div>

          <div className="header-right">
            <div className="header-actions">
              <div className="icon-badge-btn">
                <Bell size={20} />
                <span className="badge"></span>
              </div>
              <div className="icon-badge-btn">
                <Settings size={20} />
              </div>
            </div>
          </div>
        </header>

        {/* Dynamic Content */}
        <div className="page-content">
          {children}
        </div>
      </main>

      {/* Command Bar Overlay */}
      {commandBarOpen && (
        <div className="command-bar-overlay" onClick={() => setCommandBarOpen(false)}>
          <div className="glass-card command-bar animate-fade-in" onClick={e => e.stopPropagation()}>
            <div className={clsx("cb-input-wrapper", loading && "cb-loading")}>
              <div className={clsx("cb-icon-box", loading && "grad-ai animate-shimmer")}>
                <Zap className={clsx(loading ? "text-white" : "text-primary")} size={20} />
              </div>
              <input
                type="text"
                autoFocus
                placeholder="Bạn cần tôi giúp gì hôm nay?"
                className="cb-input"
                value={aiPrompt}
                onChange={e => setAiPrompt(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && handleSendCommand()}
                disabled={loading}
              />
              {loading ? (
                <div className="flex items-center px-2">
                  <Loader2 className="animate-spin text-primary" size={20} />
                </div>
              ) : (
                <button className="cb-send-btn" onClick={handleSendCommand}>
                  <Send size={18} />
                </button>
              )}
            </div>

            {aiResponse && (
              <div className="cb-response animate-fade-in">
                <div className="response-header">
                  <div className="ai-badge grad-ai animate-shimmer">
                    <Zap size={10} className="text-white" />
                    <span>AI Brain</span>
                  </div>
                </div>
                <div className="response-body">
                  <p className="message-content">{aiResponse.message}</p>
                </div>
              </div>
            )}

            <div className="cb-hints">
              {aiResponse?.action ? (
                <div className="action-tag">
                  <ShieldCheck size={14} className="text-success" />
                  <span>Action: {aiResponse.action.type.replace('_', ' ')}</span>
                </div>
              ) : (
                <div className="hint-text">
                  <Search size={12} />
                  <span>Gợi ý: "Tạo phòng Sale", "Mở sơ đồ tổ chức"</span>
                </div>
              )}
              <div className="kbd-hint">
                <span className="kbd">ESC</span> to close
              </div>
            </div>
          </div>
        </div>
      )}

      <style>{`
        .layout-container {
          display: flex;
          height: 100vh;
          background: hsl(var(--bg-deep));
          color: white;
          overflow: hidden;
        }

        /* Sidebar Styling */
        .sidebar {
          width: 280px;
          border-right: 1px solid var(--border);
          display: flex;
          flex-direction: column;
          background: hsl(var(--bg-card));
          box-shadow: 10px 0 30px -15px rgba(0,0,0,0.5);
          z-index: 10;
        }

        .sidebar-header {
          padding: 2rem 1.5rem;
          border-bottom: 1px solid var(--border);
        }

        .logo-box {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          font-size: 1.25rem;
          font-weight: 800;
          letter-spacing: -0.04em;
          background: linear-gradient(135deg, #fff 50%, hsl(var(--primary)));
          -webkit-background-clip: text;
          background-clip: text;
          -webkit-text-fill-color: transparent;
        }

        .sidebar-nav {
          flex: 1;
          padding: 2rem 0.75rem;
          overflow-y: auto;
        }

        .nav-section {
          margin-bottom: 2.5rem;
        }

        .section-label {
          display: block;
          padding: 0 1rem;
          font-size: 0.7rem;
          font-weight: 700;
          text-transform: uppercase;
          color: hsl(var(--text-muted));
          margin-bottom: 1rem;
          letter-spacing: 0.1em;
        }

        .sidebar-item {
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 0.875rem 1.25rem;
          border-radius: 0.875rem;
          color: hsl(var(--text-dim));
          text-decoration: none;
          font-size: 0.9375rem;
          font-weight: 500;
          transition: var(--transition-smooth);
          margin-bottom: 0.5rem;
        }

        .sidebar-item:hover {
          background: rgba(255, 255, 255, 0.04);
          color: white;
          transform: translateX(4px);
        }

        .sidebar-item.active {
          background: hsl(var(--primary) / 0.1);
          color: hsl(var(--primary));
          box-shadow: inset 0 0 0 1px hsl(var(--primary) / 0.2);
        }

        .sidebar-footer {
          padding: 1.25rem 1rem;
          border-top: 1px solid var(--border);
          background: rgba(0,0,0,0.15);
        }

        .user-info {
          display: flex;
          align-items: center;
          gap: 0.875rem;
          min-width: 0; /* Allow text truncation if needed */
        }

        .user-avatar {
          width: 36px;
          height: 36px;
          border-radius: 10px;
          background: linear-gradient(135deg, hsl(var(--primary)) 0%, hsl(var(--accent-purple)) 100%);
          display: flex;
          align-items: center;
          justify-content: center;
          font-weight: 700;
          font-size: 0.8125rem;
          flex-shrink: 0;
          box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        }

        .user-meta {
          display: flex;
          flex-direction: column;
          gap: 1px;
          overflow: hidden;
        }

        .user-name { 
          font-size: 0.875rem; 
          font-weight: 600; 
          white-space: nowrap; 
          overflow: hidden; 
          text-overflow: ellipsis; 
        }
        .user-role { 
          font-size: 0.725rem; 
          color: hsl(var(--text-muted)); 
          font-weight: 500;
        }

        /* Main Viewport Styling */
        .main-viewport {
          flex: 1;
          display: flex;
          flex-direction: column;
          background: hsl(var(--bg-deep));
          position: relative;
        }

        .top-header {
          height: 72px;
          border-bottom: 1px solid var(--border);
          display: grid;
          grid-template-columns: 1fr auto 1fr;
          align-items: center;
          padding: 0 2rem;
          background: hsl(var(--bg-deep) / 0.85);
          backdrop-filter: blur(12px);
          z-index: 50;
        }

        .header-center {
          display: flex;
          justify-content: center;
        }

        .header-right {
          display: flex;
          justify-content: flex-end;
        }

        .search-trigger {
          display: flex;
          align-items: center;
          gap: 0.875rem;
          background: hsl(var(--bg-card));
          padding: 0.625rem 1.25rem;
          border-radius: 0.875rem;
          border: 1px solid var(--border);
          color: hsl(var(--text-muted));
          font-size: 0.875rem;
          cursor: pointer;
          width: 420px;
          transition: var(--transition-smooth);
        }

        .search-trigger:hover { 
          border-color: hsl(var(--primary) / 0.4);
          background: hsl(var(--bg-input));
          transform: translateY(-1px);
          box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        }

        /* Command Bar Components */
        .command-bar-overlay {
          position: fixed;
          top: 0;
          left: 0;
          width: 100vw;
          height: 100vh;
          background: rgba(0,0,0,0.75);
          backdrop-filter: blur(10px);
          z-index: 1000;
          display: flex;
          justify-content: center;
          padding-top: 15vh;
        }

        .command-bar {
          width: 100%;
          max-width: 680px;
          overflow: hidden;
          background: linear-gradient(180deg, hsl(var(--bg-card) / 0.98), hsl(var(--bg-deep) / 0.99));
          border: 1px solid hsl(var(--primary) / 0.25);
          box-shadow: 0 30px 60px -12px rgba(0,0,0,0.8), 0 0 20px -5px hsl(var(--primary) / 0.15);
        }

        .cb-input-wrapper {
          padding: 1.75rem;
          display: flex;
          align-items: center;
          gap: 1.5rem;
          border-bottom: 1px solid var(--border);
        }

        .cb-icon-box {
          width: 44px;
          height: 44px;
          border-radius: 12px;
          background: hsl(var(--bg-input));
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
          transition: var(--transition-smooth);
        }

        .cb-input {
          flex: 1;
          background: none;
          border: none;
          color: white;
          font-size: 1.125rem;
          font-weight: 500;
          outline: none;
          height: 44px;
        }

        .cb-hints {
          padding: 1rem 1.75rem;
          display: flex;
          justify-content: space-between;
          align-items: center;
          background: rgba(0,0,0,0.15);
          min-height: 48px;
        }

        .hint-text, .action-tag {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          font-size: 0.75rem;
          color: hsl(var(--text-muted));
        }

        .action-tag { color: hsl(var(--success)); font-weight: 600; }

        .kbd-hint { font-size: 0.7rem; color: hsl(var(--text-muted)); }
        .kbd {
          background: var(--border-bright);
          padding: 2px 6px;
          border-radius: 4px;
          color: white;
          font-family: monospace;
          margin-right: 4px;
        }

        .cb-send-btn {
          width: 36px;
          height: 36px;
          border-radius: 8px;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--border-bright);
          color: hsl(var(--text-muted));
          transition: var(--transition-smooth);
          border: none;
          cursor: pointer;
        }

        .cb-send-btn:hover {
          background: hsl(var(--primary));
          color: white;
          transform: scale(1.05);
        }
      `}</style>
    </div>
  );
};

// Helper Sub-component
const SidebarLink = ({ to, icon, label }: { to: string; icon: React.ReactNode; label: string }) => (
  <NavLink
    to={to}
    className={({ isActive }) => clsx("sidebar-item", isActive && "active")}
  >
    {icon}
    <span>{label}</span>
  </NavLink>
);

export default MainLayout;
