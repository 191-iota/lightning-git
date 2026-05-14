-- Drop tables in correct order (respecting foreign keys)
DROP TABLE IF EXISTS project_members CASCADE;
DROP TABLE IF EXISTS task CASCADE;
DROP TABLE IF EXISTS project CASCADE;
DROP TABLE IF EXISTS organization_members CASCADE;
DROP TABLE IF EXISTS organization CASCADE;

DROP POLICY IF EXISTS "Profiles are viewable by everyone" ON profiles;
DROP TABLE IF EXISTS profiles CASCADE;
DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
DROP FUNCTION IF EXISTS public.handle_new_user();


CREATE TABLE profiles (
    id UUID REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
    display_name TEXT,
    github_token TEXT
);

ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;

-- Policy to allow users to read all profiles
CREATE POLICY "Profiles are viewable by everyone"
  ON profiles FOR SELECT
  USING (true);

CREATE TABLE organization (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);

-- Roles: 'owner' (full access to all org projects), 'member' (can create projects, sees only their own)
CREATE TABLE organization_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organization(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    joined_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE (org_id, user_id)
);

CREATE TABLE project (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organization(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now(),
    repo_url TEXT
);

CREATE TABLE task (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ DEFAULT now(),
    name VARCHAR NOT NULL,
    branch_name VARCHAR NOT NULL,
    task_type VARCHAR,
    project_id UUID REFERENCES project(id) ON DELETE CASCADE
);

-- Roles: 'admin' (manage project), 'member' (read + collab)
CREATE TABLE project_members (
    project_id UUID REFERENCES project(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'member')),
    joined_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (project_id, user_id)
);

-- Indexes for faster retrieval
CREATE INDEX idx_task_project_id ON task(project_id);
CREATE INDEX idx_project_members_user_id ON project_members(user_id);
CREATE INDEX idx_project_org_id ON project(org_id);
CREATE INDEX idx_organization_members_user_id ON organization_members(user_id);

-- Function to create a profile entry on user signup
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO public.profiles (id, display_name)
  VALUES (NEW.id, NEW.raw_user_meta_data->>'display_name');
  RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Trigger to call function after user creation
CREATE TRIGGER on_auth_user_created
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();

-- Create profiles for existing users who don't have one
INSERT INTO public.profiles (id, display_name)
SELECT id, raw_user_meta_data->>'display_name'
FROM auth.users
WHERE id NOT IN (SELECT id FROM public.profiles)
