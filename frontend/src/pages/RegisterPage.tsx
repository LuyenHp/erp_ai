import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import api from '../services/api';
import { Shield, Mail, Lock, User, Building, Loader2, ArrowRight } from 'lucide-react';

const RegisterPage: React.FC = () => {
    const [formData, setFormData] = useState({
        email: '',
        password: '',
        full_name: '',
        tenant_name: ''
    });
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const navigate = useNavigate();

    const handleRegister = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setError(null);

        try {
            const { data } = await api.post('/auth/register', formData);
            localStorage.setItem('token', data.token);
            localStorage.setItem('user', JSON.stringify(data.user));
            navigate('/');
            window.location.reload();
        } catch (err: any) {
            setError(err.response?.data?.message || 'Đăng ký thất bại. Vui lòng thử lại.');
        } finally {
            setLoading(false);
        }
    };

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        setFormData({ ...formData, [e.target.name]: e.target.value });
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
                    <h1 className="heading">Đăng ký ERP AI</h1>
                    <p className="subheading">Khởi tạo thực thể số cho doanh nghiệp của bạn.</p>
                </div>

                <form onSubmit={handleRegister} className="login-form">
                    <div className="input-group">
                        <label>Tên doanh nghiệp</label>
                        <div className="input-wrapper">
                            <Building size={18} className="input-icon" />
                            <input
                                name="tenant_name"
                                type="text"
                                className="glass-input"
                                placeholder="Ví dụ: Công ty Cổ phần Alpha"
                                value={formData.tenant_name}
                                onChange={handleChange}
                                required
                            />
                        </div>
                    </div>

                    <div className="input-group">
                        <label>Họ và tên Admin</label>
                        <div className="input-wrapper">
                            <User size={18} className="input-icon" />
                            <input
                                name="full_name"
                                type="text"
                                className="glass-input"
                                placeholder="Nguyễn Văn A"
                                value={formData.full_name}
                                onChange={handleChange}
                                required
                            />
                        </div>
                    </div>

                    <div className="input-group">
                        <label>Email công việc</label>
                        <div className="input-wrapper">
                            <Mail size={18} className="input-icon" />
                            <input
                                name="email"
                                type="email"
                                className="glass-input"
                                placeholder="admin@company.com"
                                value={formData.email}
                                onChange={handleChange}
                                required
                            />
                        </div>
                    </div>

                    <div className="input-group">
                        <label>Mật khẩu</label>
                        <div className="input-wrapper">
                            <Lock size={18} className="input-icon" />
                            <input
                                name="password"
                                type="password"
                                className="glass-input"
                                placeholder="Tối thiểu 8 ký tự"
                                value={formData.password}
                                onChange={handleChange}
                                required
                            />
                        </div>
                    </div>

                    {error && <div className="error-message">{error}</div>}

                    <button type="submit" className="login-btn" disabled={loading}>
                        {loading ? <Loader2 className="animate-spin" /> : (
                            <>
                                <span>Khởi tạo ngay</span>
                                <ArrowRight size={18} />
                            </>
                        )}
                    </button>
                </form>

                <div className="login-footer">
                    <p>Đã có tài khoản? <Link to="/login">Đăng nhập</Link></p>
                </div>
            </div>

            <style>{`
        /* Reusing styles from LoginPage.tsx - in a real app these would be in CSS modules or global.css */
        .login-container { height: 100vh; width: 100vw; display: flex; align-items: center; justify-content: center; position: relative; background: #05080f; overflow: hidden; }
        .login-blur-blobs .blob { position: absolute; filter: blur(80px); opacity: 0.15; z-index: 0; border-radius: 50%; }
        .blob-1 { width: 400px; height: 400px; background: hsl(var(--primary)); top: -100px; right: -100px; }
        .blob-2 { width: 300px; height: 300px; background: hsl(var(--accent-purple)); bottom: -50px; left: -50px; }
        .login-card { width: 100%; max-width: 480px; padding: 2.5rem; z-index: 10; }
        .login-header { text-align: center; margin-bottom: 2rem; }
        .logo-icon { width: 64px; height: 64px; background: rgba(255, 255, 255, 0.05); border-radius: 1rem; display: flex; align-items: center; justify-content: center; margin: 0 auto 1.5rem; border: 1px solid var(--glass-border); }
        .icon-gradient { color: hsl(var(--primary)); }
        .subheading { color: hsl(var(--text-dim)); font-size: 0.9375rem; margin-top: 0.5rem; }
        .login-form { display: flex; flex-direction: column; gap: 1rem; }
        .input-group label { display: block; font-size: 0.8125rem; font-weight: 500; color: hsl(var(--text-dim)); margin-bottom: 0.4rem; }
        .input-wrapper { position: relative; }
        .input-icon { position: absolute; left: 1rem; top: 50%; transform: translateY(-50%); color: hsl(var(--text-muted)); }
        .glass-input { width: 100%; padding: 0.75rem 1rem 0.75rem 2.75rem; border-radius: 0.75rem; }
        .error-message { background: rgba(220, 38, 38, 0.1); color: hsl(var(--error)); padding: 0.75rem; border-radius: 0.5rem; font-size: 0.8125rem; border: 1px solid rgba(220, 38, 38, 0.2); }
        .login-btn { background: linear-gradient(135deg, hsl(var(--primary)), hsl(var(--accent-purple))); color: white; border: none; padding: 0.875rem; border-radius: 0.75rem; font-weight: 600; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 0.5rem; transition: var(--transition-smooth); margin-top: 1rem; }
        .login-btn:hover:not(:disabled) { transform: translateY(-2px); filter: brightness(1.1); box-shadow: 0 10px 20px -5px rgba(59, 130, 246, 0.3); }
        .login-btn:disabled { opacity: 0.7; cursor: not-allowed; }
        .login-footer { margin-top: 1.5rem; text-align: center; font-size: 0.875rem; color: hsl(var(--text-dim)); }
        .login-footer a { color: hsl(var(--primary)); text-decoration: none; font-weight: 500; }
        .login-footer a:hover { text-decoration: underline; }
      `}</style>
        </div>
    );
};

export default RegisterPage;
