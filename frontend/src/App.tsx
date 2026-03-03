import { Routes, Route, Navigate, NavLink } from 'react-router-dom'
import { useState, useEffect } from 'react'
import LoginPage from './pages/LoginPage'
import RegisterPage from './pages/RegisterPage'
import MainLayout from './components/MainLayout'
import OrgTreePage from './pages/iam/OrgTreePage'
import RolesPage from './pages/iam/RolesPage'

// Dashboard Home Sub-component
const Home = () => (
    <div className="p-10 animate-fade-in max-w-7xl mx-auto">
        <div className="flex justify-between items-end mb-10">
            <div>
                <h1 className="heading grad-text" style={{ fontSize: '2.5rem', lineHeight: '1.2' }}>Chào buổi sáng,</h1>
                <p className="text-dim text-lg">Chào mừng bạn quay lại, <span className="text-white font-semibold">ERP Admin</span></p>
            </div>
            <div className="glass-card px-4 py-2 flex items-center gap-2 text-xs text-dim mb-2" style={{ borderRadius: '2rem' }}>
                <div className="w-2 h-2 rounded-full bg-success animate-pulse"></div>
                System Active: AWS-VN-01
            </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-8">
            <StatCard label="Nhân sự" value="124" trend="+3%" />
            <StatCard label="Phòng ban" value="12" trend="0" />
            <StatCard label="Yêu cầu chờ duyệt" value="5" color="hsl(var(--warning))" />
        </div>

        <div className="glass-card p-12 flex flex-col items-center justify-center border-dashed border-2" style={{ height: '320px' }}>
            <p className="text-muted mb-4 text-center">Biểu đồ phân tích AI & Tài chính đang được khởi tạo...</p>
            <div className="flex gap-2">
                <div className="w-8 h-1 bg-primary rounded-full opacity-20 animate-pulse"></div>
                <div className="w-12 h-1 bg-primary rounded-full opacity-40 animate-pulse" style={{ animationDelay: '0.2s' }}></div>
                <div className="w-8 h-1 bg-primary rounded-full opacity-20 animate-pulse" style={{ animationDelay: '0.4s' }}></div>
            </div>
        </div>
    </div>
)

const StatCard = ({ label, value, trend, color }: any) => (
    <div className="glass-card p-6">
        <span className="text-xs text-muted block mb-2 uppercase tracking-wider font-semibold">{label}</span>
        <div className="flex justify-between items-end">
            <span className="heading" style={{ fontSize: '1.75rem', color: color || 'white' }}>{value}</span>
            {trend && <span className="text-xs" style={{ color: 'hsl(var(--success))' }}>{trend}</span>}
        </div>
    </div>
)

function App() {
    const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null)

    useEffect(() => {
        const token = localStorage.getItem('token')
        setIsAuthenticated(!!token)
    }, [])

    if (isAuthenticated === null) return null

    return (
        <div className="min-h-screen">
            <Routes>
                <Route path="/login" element={!isAuthenticated ? <LoginPage /> : <Navigate to="/" />} />
                <Route path="/register" element={!isAuthenticated ? <RegisterPage /> : <Navigate to="/" />} />

                {/* Protected Dashboard Routes */}
                <Route
                    path="/*"
                    element={
                        isAuthenticated ? (
                            <MainLayout>
                                <Routes>
                                    <Route index element={<Home />} />
                                    <Route path="iam/org" element={<OrgTreePage />} />
                                    <Route path="iam/roles" element={<RolesPage />} />
                                    <Route path="iam/approvals" element={<div className="p-10"><h1>Duyệt phân quyền (HITL)</h1><p className="text-muted">Tính năng đang được xây dựng...</p></div>} />
                                    <Route path="*" element={<Navigate to="/" />} />
                                </Routes>
                            </MainLayout>
                        ) : <Navigate to="/login" replace />
                    }
                />
            </Routes>
        </div>
    )
}

export default App
