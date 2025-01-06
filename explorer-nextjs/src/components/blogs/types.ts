type Content = { type: string; text: string };

type BlogArticle = {
  title: string;
  label: string;
  description: string;
  image: string;
  icon: string;
  attributes: {
    blogAuthors: string[];
    date: Date;
    readingTime: number;
  };
  overview: {
    content: Content[];
  };
  sections: {
    id: string;
    heading: string;
    text: Content[];
  }[];
};

export type BlogArticleWithLink = BlogArticle & {
  link: string;
};

export default BlogArticle;
