import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';
import { Shield, Mail, Lock, Loader2, ArrowRight } from 'lucide-react';

const LoginPage: React.FC = () => {
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const navigate = useNavigate();

    const handleLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setError(null);

        try {
            const { data } = await api.post('/auth/login', { email, password });
            localStorage.setItem('token', data.token);
            localStorage.setItem('user', JSON.stringify(data.user));
            navigate('/');
            window.location.reload(); // Quick way to trigger auth state in App.tsx
        } catch (err: any) {
            setError(err.response?.data?.message || 'Login failed. Please check your credentials.');
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="login-container">
            <div className="login-blur-blobs">
                <div className="blob blob-1"></div>
                <div className="blob blob-2"></div>
            </div>

            <div className="glass-card login-card animate-fade-in">
                <div className="login-header">
                    <div className="logo-icon">
                        <Shield size={32} className="icon-gradient" />
                    </div>
                    <h1 className="heading">ERP AI-Native</h1>
                    <p className="subheading">Chào mừng trở lại. Đăng nhập để tiếp tục.</p>
                </div>

                <form onSubmit={handleLogin} className="login-form">
                    <div className="input-group">
                        <label>Email</label>
                        <div className="input-wrapper">
                            <Mail size={18} className="input-icon" />
                            <input
                                type="email"
                                className="glass-input"
                                placeholder="email@example.com"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                required
                            />
                        </div>
                    </div>

                    <div className="input-group">
                        <label>Mật khẩu</label>
                        <div className="input-wrapper">
                            <Lock size={18} className="input-icon" />
                            <input
                                type="password"
                                className="glass-input"
                                placeholder="••••••••"
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                required
                            />
                        </div>
                    </div>

                    {error && <div className="error-message">{error}</div>}

                    <button type="submit" className="login-btn" disabled={loading}>
                        {loading ? <Loader2 className="animate-spin" /> : (
                            <>
                                <span>Đăng nhập</span>
                                <ArrowRight size={18} />
                            </>
                        )}
                    </button>
                </form>

                <div className="login-footer">
                    <p>Chưa có tài khoản? <Link to="/register">Đăng ký doanh nghiệp</Link></p>
                </div>
            </div>

            <style>{`
        .login-container {
          height: 100vh;
          width: 100vw;
          display: flex;
          align-items: center;
          justify-content: center;
          position: relative;
          background: #05080f;
          overflow: hidden;
        }

        .login-blur-blobs .blob {
          position: absolute;
          filter: blur(80px);
          opacity: 0.15;
          z-index: 0;
          border-radius: 50%;
        }

        .blob-1 {
          width: 400px;
          height: 400px;
          background: hsl(var(--primary));
          top: -100px;
          right: -100px;
        }

        .blob-2 {
          width: 300px;
          height: 300px;
          background: hsl(var(--accent-purple));
          bottom: -50px;
          left: -50px;
        }

        .login-card {
          width: 100%;
          max-width: 420px;
          padding: 2.5rem;
          z-index: 10;
        }

        .login-header {
          text-align: center;
          margin-bottom: 2rem;
        }

        .logo-icon {
          width: 64px;
          height: 64px;
          background: rgba(255, 255, 255, 0.05);
          border-radius: 1rem;
          display: flex;
          align-items: center;
          justify-content: center;
          margin: 0 auto 1.5rem;
          border: 1px solid var(--glass-border);
        }

        .icon-gradient {
          color: hsl(var(--primary));
        }

        .subheading {
          color: hsl(var(--text-dim));
          font-size: 0.9375rem;
          margin-top: 0.5rem;
        }

        .login-form {
          display: flex;
          flex-direction: column;
          gap: 1.25rem;
        }

        .input-group label {
          display: block;
          font-size: 0.8125rem;
          font-weight: 500;
          color: hsl(var(--text-dim));
          margin-bottom: 0.5rem;
        }

        .input-wrapper {
          position: relative;
        }

        .input-icon {
          position: absolute;
          left: 1rem;
          top: 50%;
          transform: translateY(-50%);
          color: hsl(var(--text-muted));
        }

        .glass-input {
          width: 100%;
          padding: 0.75rem 1rem 0.75rem 2.75rem;
          border-radius: 0.75rem;
        }

        .error-message {
          background: rgba(220, 38, 38, 0.1);
          color: hsl(var(--error));
          padding: 0.75rem;
          border-radius: 0.5rem;
          font-size: 0.8125rem;
          border: 1px solid rgba(220, 38, 38, 0.2);
        }

        .login-btn {
          background: linear-gradient(135deg, hsl(var(--primary)), hsl(var(--accent-purple)));
          color: white;
          border: none;
          padding: 0.875rem;
          border-radius: 0.75rem;
          font-weight: 600;
          cursor: pointer;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          transition: var(--transition-smooth);
        }

        .login-btn:hover:not(:disabled) {
          transform: translateY(-2px);
          filter: brightness(1.1);
          box-shadow: 0 10px 20px -5px rgba(59, 130, 246, 0.3);
        }

        .login-btn:disabled {
          opacity: 0.7;
          cursor: not-allowed;
        }

        .login-footer {
          margin-top: 2rem;
          text-align: center;
          font-size: 0.875rem;
          color: hsl(var(--text-dim));
        }

        .login-footer a {
          color: hsl(var(--primary));
          text-decoration: none;
          font-weight: 500;
        }

        .login-footer a:hover {
          text-decoration: underline;
        }
      `}</style>
        </div>
    );
};

export default LoginPage;
